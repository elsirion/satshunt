use crate::models::{AdminScan, DailyScanCount};
use maud::{html, Markup};

pub fn admin_scans(
    scans: &[AdminScan],
    daily_counts: &[DailyScanCount],
    current_page: i64,
    total_pages: i64,
    days: i64,
) -> Markup {
    let max_count = daily_counts
        .iter()
        .map(|d| d.count)
        .max()
        .unwrap_or(1)
        .max(1);

    html! {
        div class="mb-8" {
            div class="flex justify-between items-center mb-8" {
                h1 class="text-4xl font-black text-primary" style="letter-spacing: -0.02em;" {
                    "SCAN LOG"
                }
            }

            // Timeframe selector
            div class="flex flex-wrap gap-2 mb-6" {
                @for &d in &[30, 60, 90] {
                    @if d == days {
                        a href={"/admin/scans?days=" (d)} class="btn-brutal-fill" {
                            (d) " DAYS"
                        }
                    } @else {
                        a href={"/admin/scans?days=" (d)} class="btn-brutal" {
                            (d) " DAYS"
                        }
                    }
                }
                @if days == 0 {
                    a href="/admin/scans?days=0" class="btn-brutal-fill" { "ALL" }
                } @else {
                    a href="/admin/scans?days=0" class="btn-brutal" { "ALL" }
                }
            }

            // Daily scan graph
            div class="card-brutal mb-8" {
                h2 class="text-xl font-black text-primary mb-4" {
                    "DAILY SCANS"
                }
                @let num_days = daily_counts.len();
                @let label_interval = if num_days <= 14 { 1 } else if num_days <= 31 { 7 } else if num_days <= 90 { 14 } else { 30 };
                div style="overflow-x: auto;" {
                    // Bars
                    div style="display: flex; align-items: flex-end; height: 180px; gap: 1px; min-width: 100%;" {
                        @for day in daily_counts {
                            @let bar_height = if max_count > 0 { (day.count as f64 / max_count as f64 * 100.0).max(1.0) } else { 1.0 };
                            div
                                style={"flex: 1; min-width: 2px; height: " (bar_height as i64) "%; background: var(--highlight);"}
                                title={(day.date) ": " (day.count) " scans"} {}
                        }
                    }
                    // Date labels
                    div style="display: flex; gap: 1px; min-width: 100%;" {
                        @for (i, day) in daily_counts.iter().enumerate() {
                            div style="flex: 1; min-width: 2px; text-align: center; overflow: visible; position: relative;" {
                                @if i % label_interval == 0 {
                                    span class="mono text-muted select-none" style="font-size: 0.55rem; position: absolute; left: 0; white-space: nowrap;" {
                                        (&day.date[5..])
                                    }
                                }
                            }
                        }
                    }
                    // Spacer for labels
                    div style="height: 14px;" {}
                }
            }

            // Scan list table
            @if scans.is_empty() {
                div class="card-brutal-inset text-center" style="padding: 3rem;" {
                    div class="text-6xl mb-6 text-muted" {
                        i class="fa-solid fa-nfc-magnifying-glass" {}
                    }
                    h3 class="text-2xl font-black text-primary mb-3" { "NO SCANS" }
                    p class="text-secondary mb-8 font-bold" {
                        "NO SCANS RECORDED YET."
                    }
                }
            } @else {
                div class="card-brutal overflow-x-auto" {
                    table class="w-full text-sm" style="border-collapse: collapse;" {
                        thead {
                            tr style="border-bottom: 3px solid var(--accent-muted);" {
                                th class="text-left py-3 px-3 font-black text-primary" { "TIMESTAMP" }
                                th class="text-left py-3 px-3 font-black text-primary" { "LOCATION" }
                                th class="text-left py-3 px-3 font-black text-primary" { "SCANNER" }
                                th class="text-right py-3 px-3 font-black text-primary" { "SATS" }
                            }
                        }
                        tbody {
                            @for scan in scans {
                                tr style="border-bottom: 1px solid var(--accent-muted);" {
                                    td class="py-2 px-3 mono text-secondary" style="white-space: nowrap;" {
                                        (scan.scanned_at.format("%Y-%m-%dT%H:%M:%S").to_string())
                                    }
                                    td class="py-2 px-3" {
                                        a href={"/locations/" (scan.location_id)} class="font-bold text-primary hover:text-highlight" {
                                            (scan.location_name)
                                        }
                                        span class="text-xs text-muted ml-1" {
                                            "by " (scan.creator_display_name())
                                        }
                                    }
                                    td class="py-2 px-3 font-bold mono" {
                                        (scan.scanner_display_name())
                                    }
                                    td class="py-2 px-3 text-right mono" {
                                        @if scan.claimed_at.is_some() {
                                            span class="text-highlight font-bold" {
                                                (scan.sats_claimed())
                                            }
                                        } @else {
                                            span class="text-muted" { "-" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Pagination
                @if total_pages > 1 {
                    div class="flex justify-center items-center gap-2 mt-6" {
                        @if current_page > 1 {
                            a href={"/admin/scans?page=" (current_page - 1) "&days=" (days)} class="btn-brutal" {
                                i class="fa-solid fa-chevron-left mr-1" {}
                                "PREV"
                            }
                        }
                        span class="px-4 py-2 font-bold mono text-secondary" {
                            (current_page) " / " (total_pages)
                        }
                        @if current_page < total_pages {
                            a href={"/admin/scans?page=" (current_page + 1) "&days=" (days)} class="btn-brutal" {
                                "NEXT"
                                i class="fa-solid fa-chevron-right ml-1" {}
                            }
                        }
                    }
                }
            }
        }
    }
}
