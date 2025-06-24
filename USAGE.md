# Vanity Address Generator - Usage Guide

## Quick Start

```bash
# Generate a vanity address starting with "Sol"
vanity --base 11111111111111111111111111111112 \
       --owner So11111111111111111111111111111111111111112 \
       --target Sol

# Run benchmark to test performance
vanity --benchmark --num-gpus 1
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

## Output

When a vanity address is found:
```
✨ Found matching address: SolABC123...xyz
Found seed: 9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM
Full address: SolABC123def456ghi789jkl012mno345pqr678stu
```

The seed can be used to recreate the vanity PDA in your Solana program.

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