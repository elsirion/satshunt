# ⚡ SatsHunt

A real-world treasure hunting game powered by Bitcoin's Lightning Network. Hide satoshis in physical locations using NFC stickers with LNURL-withdraw, and let others discover and claim them!

## 🎯 Features

- **📍 Location-Based Treasure Hunt**: Create treasure locations with coordinates, photos, and descriptions
- **⚡ Lightning-Powered**: Uses LNURL-withdraw on NFC tags for instant sat claims
- **🗺️ Interactive Map**: View all treasure locations and their current sat balances
- **💰 Auto-Refill**: Locations automatically refill from a community donation pool
- **📱 Mobile-Friendly**: Responsive design with dark theme
- **🏷️ NFC Setup**: One-time QR codes for easy NFC sticker writing

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ ([rustup.rs](https://rustup.rs/))
- SQLite (usually pre-installed on most systems)

### Installation

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd satshunt
   ```

2. **Copy environment configuration**
   ```bash
   cp .env.example .env
   ```

3. **Edit `.env` with your settings**
   ```bash
   # Server configuration
   HOST=127.0.0.1
   PORT=3000
   BASE_URL=http://localhost:3000  # Update for production

   # Database
   DATABASE_URL=sqlite:satshunt.db

   # Lightning configuration
   LIGHTNING_NETWORK=regtest  # Change to mainnet/testnet for production
   LIGHTNING_DATA_DIR=./lightning_data

   # Application settings
   REFILL_RATE_SATS_PER_HOUR=100
   MAX_SATS_PER_LOCATION=10000
   UPLOAD_DIR=./uploads
   ```

4. **Build and run**
   ```bash
   cargo build --release
   cargo run --release
   ```

5. **Open your browser**
   ```
   http://localhost:3000
   ```

## 📖 How It Works

### For Treasure Creators

1. **Add a Location**
   - Go to "Add Location" in the navigation
   - Enter coordinates (or use GPS to get your current location)
   - Upload photos to help others find it
   - Set the maximum satoshi capacity
   - Click "Create Location"

2. **Write the NFC Sticker**
   - After creating a location, you'll receive a one-time setup link
   - Scan the QR code with an NFC writing app (like Boltcard or LNbits)
   - Write the LNURL-withdraw link to your NFC sticker
   - Place the sticker at the location

3. **Monitor Your Location**
   - Watch as people discover and claim sats
   - The location will automatically refill from the donation pool

### For Treasure Hunters

1. **Browse the Map**
   - View all active treasure locations
   - Check how many sats are available at each spot
   - Plan your treasure hunt route!

2. **Find the Location**
   - Use the coordinates and photos to locate the NFC sticker
   - Look for creative hiding spots!

3. **Claim Your Sats**
   - Scan the NFC tag with your Lightning wallet
   - Accept the LNURL-withdraw offer
   - Sats are instantly yours!

## 🛠️ Technology Stack

- **Backend**: Rust with [Axum](https://github.com/tokio-rs/axum) web framework
- **Database**: SQLite with [SQLx](https://github.com/launchbadge/sqlx)
- **Templates**: Server-side rendering with [Maud](https://maud.lambda.xyz/)
- **Lightning**: [Blitzi](https://docs.rs/blitzi/) for Lightning Network integration
- **Styling**: Tailwind CSS + Flowbite components (dark theme)
- **Maps**: Leaflet.js for interactive maps

## 🗂️ Project Structure

```
satshunt/
├── src/
│   ├── main.rs              # Application entry point
│   ├── db.rs                # Database layer
│   ├── models.rs            # Data models
│   ├── lightning.rs         # Lightning/LNURL integration
│   ├── refill.rs            # Background refill service
│   ├── handlers/
│   │   ├── pages.rs         # Page route handlers
│   │   └── api.rs           # API route handlers
│   └── templates/
│       ├── layout.rs        # Base HTML layout
│       ├── home.rs          # Landing page
│       ├── map.rs           # Map view
│       ├── new_location.rs  # Location creation form
│       ├── location_detail.rs # Location details
│       └── nfc_setup.rs     # NFC setup instructions
├── migrations/
│   └── 001_init.sql         # Database schema
├── static/                  # Static assets (CSS, JS, images)
└── uploads/                 # User-uploaded location photos
```

## 🔧 Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `HOST` | Server bind address | `127.0.0.1` |
| `PORT` | Server port | `3000` |
| `BASE_URL` | Public URL for LNURL callbacks | `http://localhost:3000` |
| `DATABASE_URL` | SQLite database path | `sqlite:satshunt.db` |
| `REFILL_RATE_SATS_PER_HOUR` | Sats added per location per hour | `100` |
| `UPLOAD_DIR` | Directory for photo uploads | `./uploads` |

### Database

The database is automatically initialized with SQLx migrations on first run. The schema includes:

- **locations** - Treasure location data
- **photos** - Location photos
- **donation_pool** - Community donation pool (singleton)
- **scans** - Withdrawal history

## 🌐 API Endpoints

### Pages
- `GET /` - Landing page with stats
- `GET /map` - Interactive treasure map
- `GET /locations/new` - Create new location
- `GET /locations/:id` - Location details
- `GET /setup/:write_token` - NFC setup page (one-time use)

### API
- `POST /api/locations` - Create a new location (multipart/form-data)
- `GET /api/lnurlw/:location_id` - LNURL-withdraw endpoint
- `GET /api/lnurlw/:location_id/callback` - LNURL withdrawal callback
- `GET /api/stats` - Global statistics (JSON)

## 🔐 Security Notes

⚠️ **Important**: This is a demonstration application. For production use:

1. **Lightning Integration**: The current Lightning implementation is simplified. Integrate with a real Lightning node using Blitzi properly.
2. **HTTPS**: Use HTTPS in production for secure LNURL callbacks.
3. **Rate Limiting**: Implement rate limiting to prevent abuse.
4. **Authentication**: Add authentication for location management.
5. **Input Validation**: Enhanced validation for all user inputs.
6. **File Upload Security**: Implement virus scanning and file type validation.

## 🤝 Contributing

Contributions are welcome! Areas for improvement:

- Full Lightning node integration with Blitzi
- Real invoice parsing and payment execution
- User authentication and location ownership
- Mobile companion app for NFC writing
- Enhanced map features (clustering, search, filters)
- Admin dashboard for donation pool management
- Multi-language support

## 📝 License

This project is open source. See LICENSE file for details.

## 💡 Ideas for Enhancement

- **Gamification**: Leaderboards, achievements, treasure hunter profiles
- **Location Types**: Different treasure types (daily, weekly, one-time)
- **Social Features**: Comments, ratings, photo verification
- **Donation Management**: Direct donation interface, sponsorship
- **Analytics**: Heatmaps, popular locations, success rates
- **Mobile App**: Native iOS/Android apps with NFC support

## 🆘 Troubleshooting

### Build Issues

**Problem**: `jemalloc` build fails
```bash
# Add optimization to profile in Cargo.toml (already included)
[profile.dev.package.tikv-jemalloc-sys]
opt-level = 1
```

**Problem**: Database migration errors
```bash
# Delete database and restart (development only)
rm satshunt.db
cargo run
```

### Runtime Issues

**Problem**: Can't access from other devices
```bash
# Change HOST in .env to bind to all interfaces
HOST=0.0.0.0
# Update BASE_URL to your machine's IP
BASE_URL=http://192.168.1.100:3000
```

**Problem**: Upload directory permission errors
```bash
mkdir -p uploads
chmod 755 uploads
```

## 🎮 Getting Started for Development

1. **Run in development mode**
   ```bash
   cargo run
   ```

2. **Watch for changes** (install cargo-watch)
   ```bash
   cargo install cargo-watch
   cargo watch -x run
   ```

3. **Check for errors**
   ```bash
   cargo clippy
   ```

4. **Format code**
   ```bash
   cargo fmt
   ```

---

**Happy Treasure Hunting! ⚡🗺️**

For questions or issues, please open an issue on GitHub.
