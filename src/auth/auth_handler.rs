//! Module providing the `auth()` wrapper for handlers that need CookieUser.
//!
//! This wrapper extracts the CookieUser and ensures the cookie jar is always
//! returned alongside the handler's response, removing the need for handlers
//! to manually return `(user.jar, response)` tuples.
//!
//! **Note:** Handlers that need to modify the cookie jar (login, logout, register)
//! should NOT use this wrapper. They should handle the jar manually.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::auth::{auth, CookieUser};
//!
//! // Handler with auth wrapper - jar returned automatically:
//! pub async fn my_page(user: CookieUser, ...) -> impl IntoResponse {
//!     Html(content)  // No need to return user.jar!
//! }
//!
//! // In router:
//! .route("/my-page", get(auth(handlers::my_page)))
//!
//! // Handler that modifies the jar - do NOT use auth wrapper:
//! pub async fn login(user: CookieUser, Form(req): Form<LoginRequest>) -> impl IntoResponse {
//!     let jar = set_user_cookie(user.jar, &user_id);
//!     (jar, Redirect::to("/"))
//! }
//! ```
use super::CookieUser;
use crate::handlers::api::AppState;
use axum::{
    body::Body,
    extract::{FromRequest, FromRequestParts, Request},
    handler::Handler,
    response::{IntoResponse, Response},
};

use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

/// Wrapper that provides a handler with CookieUser and auto-returns the jar.
pub struct AuthHandler<F, T, M> {
    f: F,
    _marker: PhantomData<(T, M)>,
}

impl<F, T, M> Clone for AuthHandler<F, T, M>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _marker: PhantomData,
        }
    }
}

/// Create an auth-wrapped handler.
///
/// The wrapped handler receives `CookieUser` as its first argument,
/// and the cookie jar is automatically included in the response.
pub fn auth<F, T, M>(f: F) -> AuthHandler<F, T, M> {
    AuthHandler {
        f,
        _marker: PhantomData,
    }
}

// Macro to implement Handler for different arities where all args are FromRequestParts
macro_rules! impl_auth_handler {
        (
            [$($ty:ident),*]
        ) => {
            #[allow(non_snake_case, unused_mut, unused_variables)]
            impl<F, Fut, Res, $($ty,)*> Handler<(CookieUser, $($ty,)*), Arc<AppState>> for AuthHandler<F, (CookieUser, $($ty,)*), ()>
            where
                F: FnOnce(CookieUser, $($ty,)*) -> Fut + Clone + Send + Sync + 'static,
                Fut: Future<Output = Res> + Send,
                Res: IntoResponse,
                $( $ty: FromRequestParts<Arc<AppState>> + Send + 'static, )*
            {
                type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

                fn call(self, req: Request<Body>, state: Arc<AppState>) -> Self::Future {
                    Box::pin(async move {
                        let (mut parts, _body) = req.into_parts();

                        // Extract CookieUser first
                        let user = match CookieUser::from_request_parts(&mut parts, &state).await {
                            Ok(u) => u,
                            Err(rejection) => return rejection,
                        };

                        // Save the jar before moving user into the handler
                        let jar = user.jar.clone();

                        // Extract remaining parts
                        $(
                            let $ty = match $ty::from_request_parts(&mut parts, &state).await {
                                Ok(value) => value,
                                Err(rejection) => return (jar, rejection.into_response()).into_response(),
                            };
                        )*

                        // Call the handler
                        let response = (self.f)(user, $($ty,)*).await;

                        // Return response with jar
                        // If handler returns WithJar, its IntoResponse impl handles the jar
                        // Otherwise, we prepend the original jar
                        (jar, response).into_response()
                    })
                }
            }
        };
    }

/// Marker type for handlers with a body extractor as the last argument
pub struct WithBody;

// Macro for handlers where the last argument is FromRequest (body extractors)
macro_rules! impl_auth_handler_with_body {
        (
            [$($ty:ident),*], $last:ident
        ) => {
            #[allow(non_snake_case, unused_mut, unused_variables)]
            impl<F, Fut, Res, $($ty,)* $last> Handler<(CookieUser, $($ty,)* $last,), Arc<AppState>> for AuthHandler<F, (CookieUser, $($ty,)* $last,), WithBody>
            where
                F: FnOnce(CookieUser, $($ty,)* $last,) -> Fut + Clone + Send + Sync + 'static,
                Fut: Future<Output = Res> + Send,
                Res: IntoResponse,
                $( $ty: FromRequestParts<Arc<AppState>> + Send + 'static, )*
                $last: FromRequest<Arc<AppState>> + Send + 'static,
            {
                type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

                fn call(self, req: Request<Body>, state: Arc<AppState>) -> Self::Future {
                    Box::pin(async move {
                        let (mut parts, body) = req.into_parts();

                        // Extract CookieUser first
                        let user = match CookieUser::from_request_parts(&mut parts, &state).await {
                            Ok(u) => u,
                            Err(rejection) => return rejection,
                        };

                        // Save the jar before moving user into the handler
                        let jar = user.jar.clone();

                        // Extract remaining parts
                        $(
                            let $ty = match $ty::from_request_parts(&mut parts, &state).await {
                                Ok(value) => value,
                                Err(rejection) => return (jar.clone(), rejection.into_response()).into_response(),
                            };
                        )*

                        // Reconstruct request for body extraction
                        let req = Request::from_parts(parts, body);

                        // Extract body
                        let $last = match $last::from_request(req, &state).await {
                            Ok(value) => value,
                            Err(rejection) => return (jar, rejection.into_response()).into_response(),
                        };

                        // Call the handler
                        let response = (self.f)(user, $($ty,)* $last,).await;

                        // Return response with jar
                        (jar, response).into_response()
                    })
                }
            }
        };
    }

// Implement for various arities (CookieUser only, up to CookieUser + 4 args)
impl_auth_handler!([]);
impl_auth_handler!([T1]);
impl_auth_handler!([T1, T2]);
impl_auth_handler!([T1, T2, T3]);
impl_auth_handler!([T1, T2, T3, T4]);

// Implement for handlers with body extractors
impl_auth_handler_with_body!([], B1);
impl_auth_handler_with_body!([T1], B1);
impl_auth_handler_with_body!([T1, T2], B1);
impl_auth_handler_with_body!([T1, T2, T3], B1);

/// Helper to create auth wrapper for handlers with body extractors.
///
/// Use this when your handler has a body extractor (Form, Json, etc.) as
/// the last argument.
pub fn auth_body<F, T>(f: F) -> AuthHandler<F, T, WithBody> {
    AuthHandler {
        f,
        _marker: PhantomData,
    }
}
