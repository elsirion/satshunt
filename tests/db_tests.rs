use satshunt::db::Database;
use satshunt::models::AuthMethod;
use tempfile::TempDir;

async fn setup_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite:{}", db_path.display());
    let db = Database::new(&db_url).await.unwrap();
    (db, temp_dir)
}

#[tokio::test]
async fn test_create_user() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "test_hash".to_string(),
    };

    let user = db
        .create_user(
            "testuser".to_string(),
            Some("test@example.com".to_string()),
            auth,
        )
        .await
        .unwrap();

    assert_eq!(user.username, Some("testuser".to_string()));
    assert_eq!(user.email, Some("test@example.com".to_string()));
    assert!(!user.id.is_empty());
}

#[tokio::test]
async fn test_get_user_by_username() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "test_hash".to_string(),
    };

    db.create_user("findme".to_string(), None, auth)
        .await
        .unwrap();

    let found = db.get_user_by_username("findme").await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().username, Some("findme".to_string()));

    let not_found = db.get_user_by_username("nonexistent").await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_create_location() {
    let (db, _temp) = setup_test_db().await;

    // Create a user first (locations require a user_id)
    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("locowner".to_string(), None, auth)
        .await
        .unwrap();

    let location = db
        .create_location(
            "Test Treasure".to_string(),
            51.5074,
            -0.1278,
            Some("A hidden treasure in London".to_string()),
            "secret123".to_string(),
            user.id.clone(),
        )
        .await
        .unwrap();

    assert_eq!(location.name, "Test Treasure");
    assert!((location.latitude - 51.5074).abs() < 0.0001);
    assert!((location.longitude - (-0.1278)).abs() < 0.0001);
    assert_eq!(location.user_id, user.id);
    assert_eq!(location.status, "created");
    assert_eq!(location.current_msats, 0);
}

#[tokio::test]
async fn test_get_location() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("owner".to_string(), None, auth)
        .await
        .unwrap();

    let created = db
        .create_location(
            "Findable".to_string(),
            0.0,
            0.0,
            None,
            "secret".to_string(),
            user.id,
        )
        .await
        .unwrap();

    let found = db.get_location(&created.id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Findable");

    let not_found = db.get_location("nonexistent-id").await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_donation_pool_operations() {
    let (db, _temp) = setup_test_db().await;

    // Create a user and location for testing
    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("pooltest".to_string(), None, auth)
        .await
        .unwrap();
    let location = db
        .create_location(
            "Pool Test Location".to_string(),
            0.0,
            0.0,
            None,
            "secret".to_string(),
            user.id,
        )
        .await
        .unwrap();

    // Get initial location pool balance (should be 0)
    let balance = db
        .get_location_donation_pool_balance(&location.id)
        .await
        .unwrap();
    assert_eq!(balance, 0);

    // Add to pool via location-specific donation
    db.create_donation("lnbc100k1".to_string(), 100000, Some(&location.id))
        .await
        .unwrap();
    db.mark_donation_received("lnbc100k1").await.unwrap();
    let balance = db
        .get_location_donation_pool_balance(&location.id)
        .await
        .unwrap();
    assert_eq!(balance, 100000);

    // Add more via another donation
    db.create_donation("lnbc50k1".to_string(), 50000, Some(&location.id))
        .await
        .unwrap();
    db.mark_donation_received("lnbc50k1").await.unwrap();
    let balance = db
        .get_location_donation_pool_balance(&location.id)
        .await
        .unwrap();
    assert_eq!(balance, 150000);

    // Record a debit (refill used pool)
    db.record_location_pool_debit(&location.id, 30000)
        .await
        .unwrap();
    let balance = db
        .get_location_donation_pool_balance(&location.id)
        .await
        .unwrap();
    assert_eq!(balance, 120000);
}

#[tokio::test]
async fn test_update_location_msats() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("owner".to_string(), None, auth)
        .await
        .unwrap();

    let location = db
        .create_location(
            "Msat Test".to_string(),
            0.0,
            0.0,
            None,
            "secret".to_string(),
            user.id,
        )
        .await
        .unwrap();

    assert_eq!(location.current_msats, 0);

    // Update msats
    db.update_location_msats(&location.id, 50000).await.unwrap();

    let updated = db.get_location(&location.id).await.unwrap().unwrap();
    assert_eq!(updated.current_msats, 50000);
}

#[tokio::test]
async fn test_location_status_update() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("owner".to_string(), None, auth)
        .await
        .unwrap();

    let location = db
        .create_location(
            "Status Test".to_string(),
            0.0,
            0.0,
            None,
            "secret".to_string(),
            user.id,
        )
        .await
        .unwrap();

    assert_eq!(location.status, "created");

    // Update to programmed
    db.update_location_status(&location.id, "programmed")
        .await
        .unwrap();
    let loc = db.get_location(&location.id).await.unwrap().unwrap();
    assert_eq!(loc.status, "programmed");

    // Update to active
    db.update_location_status(&location.id, "active")
        .await
        .unwrap();
    let loc = db.get_location(&location.id).await.unwrap().unwrap();
    assert_eq!(loc.status, "active");
}

#[tokio::test]
async fn test_list_active_locations() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("owner".to_string(), None, auth)
        .await
        .unwrap();

    // Create 3 locations
    let loc1 = db
        .create_location(
            "Loc1".to_string(),
            0.0,
            0.0,
            None,
            "s1".to_string(),
            user.id.clone(),
        )
        .await
        .unwrap();
    let loc2 = db
        .create_location(
            "Loc2".to_string(),
            1.0,
            1.0,
            None,
            "s2".to_string(),
            user.id.clone(),
        )
        .await
        .unwrap();
    let loc3 = db
        .create_location(
            "Loc3".to_string(),
            2.0,
            2.0,
            None,
            "s3".to_string(),
            user.id.clone(),
        )
        .await
        .unwrap();

    // Initially none are active
    let active = db.list_active_locations().await.unwrap();
    assert_eq!(active.len(), 0);

    // Activate loc1 and loc3
    db.update_location_status(&loc1.id, "active").await.unwrap();
    db.update_location_status(&loc2.id, "programmed")
        .await
        .unwrap();
    db.update_location_status(&loc3.id, "active").await.unwrap();

    let active = db.list_active_locations().await.unwrap();
    assert_eq!(active.len(), 2);
}

#[tokio::test]
async fn test_record_scan() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("owner".to_string(), None, auth)
        .await
        .unwrap();

    let location = db
        .create_location(
            "Scan Test".to_string(),
            0.0,
            0.0,
            None,
            "secret".to_string(),
            user.id,
        )
        .await
        .unwrap();

    // Record a scan
    db.record_scan(&location.id, 10000, None).await.unwrap();

    // Get stats to verify
    let stats = db.get_stats().await.unwrap();
    assert_eq!(stats.total_scans, 1);
}

#[tokio::test]
async fn test_get_stats() {
    let (db, _temp) = setup_test_db().await;

    let auth = AuthMethod::Password {
        password_hash: "hash".to_string(),
    };
    let user = db
        .create_user("owner".to_string(), None, auth)
        .await
        .unwrap();

    // Create locations
    let loc1 = db
        .create_location(
            "L1".to_string(),
            0.0,
            0.0,
            None,
            "s1".to_string(),
            user.id.clone(),
        )
        .await
        .unwrap();
    let loc2 = db
        .create_location(
            "L2".to_string(),
            1.0,
            1.0,
            None,
            "s2".to_string(),
            user.id.clone(),
        )
        .await
        .unwrap();

    // Add msats and activate
    db.update_location_msats(&loc1.id, 100000).await.unwrap();
    db.update_location_msats(&loc2.id, 50000).await.unwrap();
    db.update_location_status(&loc1.id, "active").await.unwrap();
    db.update_location_status(&loc2.id, "active").await.unwrap();

    // Add to donation pool via donation
    db.create_donation("lnbc200k1".to_string(), 200000, None)
        .await
        .unwrap();
    db.mark_donation_received("lnbc200k1").await.unwrap();

    let stats = db.get_stats().await.unwrap();
    assert_eq!(stats.total_locations, 2);
    assert_eq!(stats.total_sats_available, 150); // 150000 msats = 150 sats
    assert_eq!(stats.donation_pool_sats, 200); // 200000 msats = 200 sats
}
