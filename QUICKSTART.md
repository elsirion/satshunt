# SatsHunt - Quick Start Guide

## 🚀 Get Running in 60 Seconds

```bash
# 1. Navigate to project
cd /home/user/projects/elsirion/satshunt

# 2. Create required directories
mkdir -p uploads lightning_data

# 3. Environment is already configured (.env file exists)
# Edit .env if needed to change port or other settings

# 4. Build and run
cargo run --release

# 5. Open browser to http://localhost:3000
```

## 📋 First Steps

### Create Your First Treasure Location

1. Open http://localhost:3000
2. Click **"Add Location"** in the navigation
3. Fill out the form:
   - **Name**: "Test Treasure"
   - **Description**: "Behind the big tree"
   - **Coordinates**: Use "Use My Location" button or enter manually
   - **Max Sats**: 1000
   - **Photos**: Upload 1-3 photos (optional)
4. Click **"Create Location"**
5. You'll be redirected to the setup page with a QR code

### View the Treasure Map

1. Click **"Map"** in navigation
2. See your location marked on the map
3. Click markers for details
4. Locations are color-coded:
   - 🟢 Green: >50% full
   - 🟡 Yellow: 20-50% full
   - 🔴 Red: <20% full

## 🎯 How the Game Works

### For Location Creators

```
Create Location → Get QR Code → Write NFC Tag → Place at Location
                                      ↓
                            Location starts at 0 sats
                                      ↓
                            Auto-refills from donation pool
                                   (100 sats/hour)
```

### For Treasure Hunters

```
Browse Map → Find Coordinates → Locate NFC Tag → Scan with Wallet → Get Sats!
```

## ⚡ Lightning Integration

Currently uses a **simplified LNURL-withdraw** implementation:

- LNURL endpoints are functional
- Returns proper LNURL-withdraw responses
- Tracks withdrawals in database
- **TODO**: Connect to real Lightning node via Blitzi

### To Add Real Lightning

Update `src/lightning.rs`:
```rust
pub async fn create_withdrawal_invoice(
    &self,
    amount_sats: i64,
    description: &str,
) -> Result<String> {
    // Initialize Blitzi client
    let blitzi = Blitzi::new().await?;

    // Create invoice
    let amount = Amount::from_sats(amount_sats as u64);
    let invoice = blitzi.lightning_invoice(amount, description).await?;

    Ok(invoice.to_string())
}
```

## 🔧 Configuration

Edit `.env` to customize:

```bash
# Change port
PORT=8080

# Change refill rate (sats per hour per location)
REFILL_RATE_SATS_PER_HOUR=200

# Change maximum sats per location
MAX_SATS_PER_LOCATION=5000

# For production, set your public URL
BASE_URL=https://satshunt.yourdomain.com
```

## 🗄️ Database

**Location**: `satshunt.db` (SQLite)

**Reset database** (development only):
```bash
rm satshunt.db satshunt.db-*
cargo run  # Will recreate with migrations
```

**View database** (requires sqlite3):
```bash
sqlite3 satshunt.db
sqlite> .tables
sqlite> SELECT * FROM locations;
sqlite> SELECT * FROM donation_pool;
sqlite> .quit
```

## 📊 Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Server setup, routing |
| `src/handlers/pages.rs` | HTML page handlers |
| `src/handlers/api.rs` | API endpoints |
| `src/templates/*.rs` | Maud HTML templates |
| `src/db.rs` | Database operations |
| `src/lightning.rs` | LNURL implementation |
| `src/refill.rs` | Background refill service |
| `migrations/001_init.sql` | Database schema |

## 🐛 Troubleshooting

### Port Already in Use
```bash
# Change PORT in .env
PORT=3001
```

### Can't Create Database
```bash
# Check permissions
chmod 755 .
mkdir -p uploads
```

### Photos Not Uploading
```bash
# Ensure upload directory exists
mkdir -p uploads
chmod 755 uploads
```

### Refill Not Working
```bash
# Check logs for errors
RUST_LOG=debug cargo run

# Verify donation pool has sats
# Pool starts at 0 - you need to add sats manually
# Or modify the initial pool in migrations/001_init.sql
```

### Add Sats to Donation Pool

Currently you need to manually update the database:
```sql
-- Via sqlite3
UPDATE donation_pool SET total_sats = 1000000 WHERE id = 1;
```

Or add an admin endpoint (TODO).

## 🔍 Testing Endpoints

### Get Stats
```bash
curl http://localhost:3000/api/stats
```

### Create Location (requires form data)
```bash
curl -X POST http://localhost:3000/api/locations \
  -F "name=Test Location" \
  -F "latitude=37.7749" \
  -F "longitude=-122.4194" \
  -F "max_sats=1000"
```

### LNURL Endpoint
```bash
curl http://localhost:3000/api/lnurlw/LOCATION_ID
```

## 📱 NFC Writing Apps

Compatible NFC writing apps:
- **Boltcard** (recommended)
- **LNbits NFC**
- **Any LNURL NFC writer**

Write this URL to the NFC tag:
```
http://localhost:3000/api/lnurlw/LOCATION_ID
```

## 🎨 Customization

### Change Theme Colors

Edit `src/templates/layout.rs`:
```rust
// Change accent color (currently yellow)
class="text-yellow-400"  // Change to blue, green, etc.
```

### Modify Refill Rate

Edit `.env`:
```bash
REFILL_RATE_SATS_PER_HOUR=500  # Refills faster!
```

### Change Map Style

Edit `src/templates/map.rs` and `new_location.rs`:
```javascript
// Use different tile provider
L.tileLayer('https://tiles.stadiamaps.com/tiles/...', {
  // Different map style
})
```

## 🚀 Production Deployment

### 1. Build Release Binary
```bash
cargo build --release
# Binary at: target/release/satshunt
```

### 2. Configure Environment
```bash
# Production .env
HOST=0.0.0.0
PORT=3000
BASE_URL=https://your-domain.com
DATABASE_URL=sqlite:/var/lib/satshunt/satshunt.db
```

### 3. Run with Systemd
```ini
[Unit]
Description=SatsHunt Treasure Hunt
After=network.target

[Service]
Type=simple
User=satshunt
WorkingDirectory=/opt/satshunt
ExecStart=/opt/satshunt/target/release/satshunt
Restart=always

[Install]
WantedBy=multi-user.target
```

### 4. Use Nginx/Caddy for HTTPS
LNURL requires HTTPS in production!

## 📈 Monitoring

### Check Logs
```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Or in production
RUST_LOG=info ./target/release/satshunt
```

### Monitor Refill Service
Logs will show:
```
Refilled location Test Treasure with 100 sats (now at 100/1000)
Total refilled: 100 sats, remaining pool: 999900 sats
```

## 🎯 Next Steps

1. ✅ Add sats to donation pool (manually via database)
2. ✅ Create test locations
3. ✅ Test LNURL endpoints
4. ⏳ Integrate real Lightning node
5. ⏳ Deploy to production with HTTPS
6. ⏳ Add user authentication
7. ⏳ Build mobile app

## 💡 Tips

- Start with a high donation pool (1M sats) for testing
- Set low refill rates initially
- Test with small max_sats values
- Use regtest or testnet Lightning for development
- Keep photos under 5MB each
- Use descriptive location names

## 🆘 Get Help

- Check the full [README.md](README.md)
- Review [FEATURES.md](FEATURES.md) for complete feature list
- Look at code comments in `src/`
- Check the database schema in `migrations/001_init.sql`

---

**Happy Treasure Hunting! ⚡🗺️**
