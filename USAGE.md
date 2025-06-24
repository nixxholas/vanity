# Vanity Address Generator - Usage Guide

## Prerequisites: Generate Solana Keypairs

Before using the vanity address generator, you'll need Solana public keys for the `--base` and `--owner` parameters.

### Option 1: Using Solana CLI (Recommended)

```bash
# Install Solana CLI if you haven't already
curl -sSfL https://release.solana.com/v1.18.4/install | sh
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"

# Generate a new keypair (this will be your EOA wallet)
solana-keygen new --outfile ~/my-wallet.json

# Get the public key from the keypair
solana-keygen pubkey ~/my-wallet.json
# Example output: 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM

# Generate another keypair for different use cases
solana-keygen new --outfile ~/my-program.json
solana-keygen pubkey ~/my-program.json
# Example output: 7N4HggYEJAtCLJdnHGCtFqfxcB5rhQCsQTze3ftYXMRj
```

### Option 2: Using Online Tools (Less Secure)

Visit [Solana Explorer Keypair Tool](https://explorer.solana.com/tools/keypair) or use:
```bash
# Quick random keypair generation (for testing only)
solana-keygen new --no-bip39-passphrase --silent --outfile /dev/stdout | head -1
```

### Option 3: Using JavaScript/TypeScript

```javascript
import { Keypair } from '@solana/web3.js';
import bs58 from 'bs58';

// Generate a new keypair
const keypair = Keypair.generate();

// Get the public key (44 characters, base58-encoded)
const publicKey = keypair.publicKey.toString();
console.log('Public Key:', publicKey);

// Get the private key (for backup)
const privateKey = bs58.encode(keypair.secretKey);
console.log('Private Key:', privateKey);
```

### Option 4: Using Phantom/Solflare Wallet

1. Install [Phantom](https://phantom.app/) or [Solflare](https://solflare.com/) wallet
2. Create a new wallet
3. Copy your wallet's public address (starts with letters/numbers, 44 characters long)

## Quick Start

```bash
# Generate a vanity address starting with "Sol" (auto-detects optimal CPU/GPU settings)
vanity --base 11111111111111111111111111111112 \
       --owner 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
       --target Sol

# Run benchmark to test your hardware performance
vanity --benchmark
```

## Parameter Explanation

### `--base <BASE>` (Required)
The base public key used in Solana Program Derived Address (PDA) generation.

**Format**: 32-byte Solana public key in base58 format (44 characters)
**Example**: `11111111111111111111111111111112` (System Program)
**Common values**:
- `11111111111111111111111111111112` - System Program
- `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` - Token Program
- Your own program's public key

### `--owner <OWNER>` (Required)  
The public key that will own/control the generated PDA.

**Format**: 32-byte Solana public key in base58 format (44 characters)
**Example**: `So11111111111111111111111111111111111111112` (Wrapped SOL)
**Typical use**: Your wallet's public key or another program's key

### `--target <TARGET>` (Required)
The desired prefix for your vanity address.

**Valid characters**: `123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz`
**Examples**:
- `Sol` - Simple 3-character prefix
- `Lucky` - 5-character word  
- `123` - Numeric prefix
- `MyToken` - Branded token prefix

**Difficulty by length**:
- 1 char: ~29 attempts (instant)
- 2 chars: ~1,700 attempts (seconds)  
- 3 chars: ~100,000 attempts (minutes)
- 4 chars: ~6 million attempts (hours)
- 5 chars: ~350 million attempts (days)
- 6+ chars: Exponentially longer

## Options

### `--case-insensitive`
Makes the search case-insensitive, significantly improving performance.

```bash
# This will match: Sol, SOL, sol, SoL, sOl, etc.
vanity --base 11111111111111111111111111111112 \
       --owner So11111111111111111111111111111111111111112 \
       --target Sol \
       --case-insensitive
```

### `--num-gpus <COUNT>`
Number of GPUs to use (default: 1).

```bash
# Use 2 GPUs
vanity --num-gpus 2 --base ... --owner ... --target ...

# Disable GPU, use CPU only
vanity --num-gpus 0 --num-cpus 8 --base ... --owner ... --target ...
```

### `--num-cpus <COUNT>`  
Number of CPU threads (default: 0 = auto-detect).

```bash
# Use 8 CPU threads
vanity --num-cpus 8 --base ... --owner ... --target ...

# Use all available CPU cores
vanity --num-cpus 0 --base ... --owner ... --target ...
```

### `--logfile <PATH>`
Save search progress to a log file.

```bash
vanity --logfile search.log --base ... --owner ... --target ...
```

## Examples

### Step-by-Step: Complete Workflow

```bash
# 1. First, generate your Solana keypairs
solana-keygen new --outfile ~/my-wallet.json
solana-keygen pubkey ~/my-wallet.json
# Output: 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM

# 2. Generate a vanity address with your keys
vanity \
  --base 11111111111111111111111111111112 \
  --owner 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --target Sol
  
# 3. The tool will output something like:
# ✨ Found matching address: SolABC123def456ghi789jkl012mno345pqr
# Found seed: 3wXXR2rBgPCsw9RszmqrHU1Bunk3GHZm7WgHLqz3pC6g
```

### Basic Vanity Address
```bash
vanity \
  --base 11111111111111111111111111111112 \
  --owner 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --target ABC
```

### Token Program Vanity Address
```bash
vanity \
  --base TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA \
  --owner 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --target Token123 \
  --case-insensitive
```

### High-Performance Search
```bash
vanity \
  --base 11111111111111111111111111111112 \
  --owner 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM \
  --target Sol \
  --num-gpus 1 \
  --num-cpus 0 \
  --case-insensitive \
  --logfile vanity_search.log
```

### Benchmark Performance
```bash
# Test GPU performance
vanity --benchmark --num-gpus 1

# Test CPU performance  
vanity --benchmark --num-cpus 8

# Test both GPU and CPU
vanity --benchmark --num-gpus 1 --num-cpus 8
```

## Performance Tips

1. **Use shorter prefixes**: Each character increases difficulty ~58x
2. **Enable case-insensitive**: Dramatically improves success rate
3. **Use Apple Silicon GPUs**: Massive speedup over CPU-only search
4. **Combine GPU + CPU**: Use both for maximum throughput
5. **Choose meaningful targets**: Common words/patterns may be easier

## Output & Next Steps

When a vanity address is found:
```
✨ Found matching address: SolABC123...xyz
Found seed: 3wXXR2rBgPCsw9RszmqrHU1Bunk3GHZm7WgHLqz3pC6g
Full address: SolABC123def456ghi789jkl012mno345pqr678stu
```

### Using the Results

The **seed** is the key piece you need. Here's how to use it:

#### In Your Solana Program (Rust)
```rust
use solana_program::pubkey::Pubkey;

// Use the found seed to create the PDA
let seed = "3wXXR2rBgPCsw9RszmqrHU1Bunk3GHZm7WgHLqz3pC6g";
let base_pubkey = Pubkey::try_from("11111111111111111111111111111112").unwrap();
let owner_pubkey = Pubkey::try_from("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM").unwrap();

let (pda, bump) = Pubkey::find_program_address(
    &[base_pubkey.as_ref(), seed.as_bytes(), owner_pubkey.as_ref()],
    &program_id,
);
```

#### In JavaScript/TypeScript
```javascript
import { PublicKey } from '@solana/web3.js';

// Use the found seed to recreate the PDA
const seed = "3wXXR2rBgPCsw9RszmqrHU1Bunk3GHZm7WgHLqz3pC6g";
const basePubkey = new PublicKey("11111111111111111111111111111112");
const ownerPubkey = new PublicKey("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM");

const [pda, bump] = PublicKey.findProgramAddressSync(
  [basePubkey.toBuffer(), Buffer.from(seed), ownerPubkey.toBuffer()],
  programId
);

console.log("Vanity PDA:", pda.toString());
```

#### Verify the Result
```bash
# You can verify the result using Solana CLI
solana address --keypair <(echo "[your-program-keypair-here]")
```

The seed can be used to deterministically recreate the vanity PDA in your Solana program.

## Troubleshooting

### "Invalid base58" Error
- Ensure your base/owner keys are valid 44-character base58 strings
- Check for typos (0, O, I, l are not valid base58 characters)

### "Target contains invalid bs58" Error  
- Remove invalid characters: 0, O, I, l
- Use only: 123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz

### Slow Performance
- Try shorter prefixes first
- Enable --case-insensitive
- Use GPU acceleration if available
- Consider the exponential difficulty scaling