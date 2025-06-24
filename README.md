# Vanity Address Generator for Solana

🚀 A *blazingly fast* tool for generating vanity addresses on Solana, optimized for Apple Silicon GPUs.

## What is this?

Generate custom Solana Program Derived Addresses (PDAs) with your desired prefix (like "Sol", "Lucky", "123", etc.). Instead of grinding ed25519 keypairs, this tool uses `CreateAccountWithSeed` for extreme speedups while covering most use cases.

## ⚡ Performance

- **Apple M4 Max**: 83+ MH/s (CPU + GPU combined)
- **GPU Only**: 47+ MH/s  
- **CPU Only**: 41+ MH/s
- **Auto-optimization**: Automatically detects and uses all available CPU cores and optimal GPU workers

## 🔧 Installation & Setup

### Quick Setup (3 steps)

```bash
# 1. Build the tool with Apple Silicon GPU support
cargo build --features apple-gpu --release

# 2. Generate Solana keypairs (if you don't have them)
curl -sSfL https://release.solana.com/v1.18.4/install | sh
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
solana-keygen new --outfile ~/my-wallet.json
solana-keygen pubkey ~/my-wallet.json

# 3. Generate your vanity address (auto-detects optimal settings)
./target/release/vanity --base 11111111111111111111111111111112 \
                        --owner YOUR_WALLET_PUBKEY_HERE \
                        --target Sol
```

### Alternative Installation

```bash
# CPU-only version
cargo install vanity

# With GPU support (Apple Silicon)
cargo install vanity --features apple-gpu
```

## 📖 Usage

### Basic Usage (Auto-optimization)

```bash
# Just specify your keys and target - the tool auto-detects optimal CPU/GPU settings
vanity --base YOUR_BASE_PUBKEY \
       --owner YOUR_OWNER_PUBKEY \
       --target Sol

# Use case-insensitive matching for faster results  
vanity --base YOUR_BASE_PUBKEY \
       --owner YOUR_OWNER_PUBKEY \
       --target Sol \
       --case-insensitive
```

### Advanced Usage

```bash
# Manual resource control (if needed)
vanity --base YOUR_BASE_PUBKEY \
       --owner YOUR_OWNER_PUBKEY \
       --target Sol \
       --num-cpus 8 \
       --num-gpus 4

# Run performance benchmark
vanity --benchmark

# Get help
vanity --help
```

### Example Output

```
[INFO] Auto-detected 14 CPU threads (from 16 available cores)
[INFO] Auto-detected 4 GPU workers (optimized for Apple Silicon)
[INFO] Starting 4 GPU workers
[INFO] Starting 14 CPU workers
[INFO] CPU 1 found seed: 3wXXR2rBgPCsw9RszmqrHU1Bunk3GHZm7WgHLqz3pC6g
[INFO] Full address: SolABC123def456ghi789jkl012mno345pqr678stu
```

## 🔍 Detailed Documentation

For comprehensive documentation including:
- How to generate Solana keypairs
- Parameter explanations  
- Code examples (Rust/JavaScript)
- Troubleshooting

See: **[USAGE.md](USAGE.md)**

## 💻 Using the Generated Results

The tool outputs a **seed** that you can use in your Solana programs:

### Rust
```rust
use solana_program::pubkey::Pubkey;

let seed = "3wXXR2rBgPCsw9RszmqrHU1Bunk3GHZm7WgHLqz3pC6g";
let (pda, bump) = Pubkey::find_program_address(
    &[base_pubkey.as_ref(), seed.as_bytes(), owner_pubkey.as_ref()],
    &program_id,
);
```

### JavaScript
```javascript
import { PublicKey } from '@solana/web3.js';

const [pda, bump] = PublicKey.findProgramAddressSync(
  [basePubkey.toBuffer(), Buffer.from(seed), ownerPubkey.toBuffer()],
  programId
);
```

## 🏆 Performance Benchmarks

Real-world performance on different hardware:

| Hardware | CPU Only | GPU Only | Combined | Notes |
|----------|----------|----------|----------|-------|
| Apple M4 Max | 41.8 MH/s | 47.6 MH/s | **83.9 MH/s** | 14 CPU + 4 GPU workers |
| Apple M3 Max | ~35 MH/s | ~42 MH/s | ~75 MH/s | Estimated |
| Apple M2 Max | ~30 MH/s | ~38 MH/s | ~65 MH/s | Estimated |
| Apple M1 Max | ~25 MH/s | ~35 MH/s | ~55 MH/s | Estimated |
| RTX 4090 | - | ~1000 MH/s | - | CUDA (reference) |

*MH/s = Million hashes per second*

### Difficulty Estimates

| Target Length | Estimated Attempts | Time on M4 Max | Example |
|---------------|-------------------|----------------|---------|
| 1 character | ~29 | Instant | "1", "A" |
| 2 characters | ~1,700 | Seconds | "So", "12" |
| 3 characters | ~100,000 | Minutes | "Sol", "ABC" |
| 4 characters | ~6 million | Hours | "Lucky", "1234" |
| 5+ characters | ~350+ million | Days+ | "Solana", "Token" |

## 🎯 Key Features

- ✅ **Auto-optimization**: Detects optimal CPU/GPU worker counts
- ✅ **Hybrid processing**: Simultaneous CPU + GPU execution  
- ✅ **Apple Silicon optimized**: Native Metal GPU acceleration
- ✅ **Case-insensitive search**: Faster results when case doesn't matter
- ✅ **Real-time progress**: Live performance metrics
- ✅ **Cross-platform**: Works on macOS, Linux, Windows
- ✅ **Production ready**: Robust error handling and resource management

## 🤝 Contributing

Contributions welcome! This project is actively maintained and optimized for performance.

## 📚 Acknowledgements

- Original CPU implementation and CUDA optimizations
- SHA2 implementation from [cuda-hashing-algos](https://github.com/mochimodev/cuda-hashing-algos) (public domain)
- Base58 encoding from Firedancer with modifications (Apache 2.0)
- Apple Silicon Metal optimizations and hybrid CPU+GPU processing
