#include <metal_stdlib>
using namespace metal;

// Device-wide constants
constant bool d_case_insensitive [[function_constant(0)]];

// Base58 alphabet
constant unsigned char alphanumeric[63] = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

// SHA256 constants
constant uint32_t K[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

// Helper function for memory operations
inline void metal_memcpy_thread(
    thread uint8_t* dst,
    thread const uint8_t* src,
    size_t size
) {
    for (size_t i = 0; i < size; ++i) {
        dst[i] = src[i];
    }
}

inline void metal_memcpy_constant(
    thread uint8_t* dst,
    constant const uint8_t* src,
    size_t size
) {
    for (size_t i = 0; i < size; ++i) {
        dst[i] = src[i];
    }
}

struct SHA256_CTX {
    uint32_t state[8];
    uint32_t count[2];
    uint8_t buffer[64];
};

// SHA256 helper functions
inline uint32_t Ch(uint32_t x, uint32_t y, uint32_t z) { return (x & y) ^ (~x & z); }
inline uint32_t Maj(uint32_t x, uint32_t y, uint32_t z) { return (x & y) ^ (x & z) ^ (y & z); }
inline uint32_t ROTRIGHT(uint32_t a, uint32_t b) { return (a >> b) | (a << (32 - b)); }
inline uint32_t EP0(uint32_t x) { return ROTRIGHT(x, 2) ^ ROTRIGHT(x, 13) ^ ROTRIGHT(x, 22); }
inline uint32_t EP1(uint32_t x) { return ROTRIGHT(x, 6) ^ ROTRIGHT(x, 11) ^ ROTRIGHT(x, 25); }
inline uint32_t SIG0(uint32_t x) { return ROTRIGHT(x, 7) ^ ROTRIGHT(x, 18) ^ (x >> 3); }
inline uint32_t SIG1(uint32_t x) { return ROTRIGHT(x, 17) ^ ROTRIGHT(x, 19) ^ (x >> 10); }

void sha256_transform(thread SHA256_CTX& ctx, thread const uint8_t* data) {
    uint32_t a, b, c, d, e, f, g, h, i, j, t1, t2, m[64];

    for (i = 0, j = 0; i < 16; ++i, j += 4)
        m[i] = (data[j] << 24) | (data[j + 1] << 16) | (data[j + 2] << 8) | (data[j + 3]);
    
    for (; i < 64; ++i)
        m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];

    a = ctx.state[0];
    b = ctx.state[1];
    c = ctx.state[2];
    d = ctx.state[3];
    e = ctx.state[4];
    f = ctx.state[5];
    g = ctx.state[6];
    h = ctx.state[7];

    for (i = 0; i < 64; ++i) {
        t1 = h + EP1(e) + Ch(e, f, g) + K[i] + m[i];
        t2 = EP0(a) + Maj(a, b, c);
        h = g;
        g = f;
        f = e;
        e = d + t1;
        d = c;
        c = b;
        b = a;
        a = t1 + t2;
    }

    ctx.state[0] += a;
    ctx.state[1] += b;
    ctx.state[2] += c;
    ctx.state[3] += d;
    ctx.state[4] += e;
    ctx.state[5] += f;
    ctx.state[6] += g;
    ctx.state[7] += h;
}

void sha256_init(thread SHA256_CTX& ctx) {
    ctx.state[0] = 0x6a09e667;
    ctx.state[1] = 0xbb67ae85;
    ctx.state[2] = 0x3c6ef372;
    ctx.state[3] = 0xa54ff53a;
    ctx.state[4] = 0x510e527f;
    ctx.state[5] = 0x9b05688c;
    ctx.state[6] = 0x1f83d9ab;
    ctx.state[7] = 0x5be0cd19;
    ctx.count[0] = ctx.count[1] = 0;
}

void sha256_update(thread SHA256_CTX& ctx, thread const uint8_t* data, size_t len) {
    for (size_t i = 0; i < len; ++i) {
        ctx.buffer[ctx.count[0]] = data[i];
        if (++ctx.count[0] == 64) {
            sha256_transform(ctx, ctx.buffer);
            ctx.count[0] = 0;
            ++ctx.count[1];
        }
    }
}

void sha256_update_constant(thread SHA256_CTX& ctx, constant const uint8_t* data, size_t len) {
    thread uint8_t temp_buffer[64];
    for (size_t i = 0; i < len; ++i) {
        ctx.buffer[ctx.count[0]] = data[i];
        if (++ctx.count[0] == 64) {
            metal_memcpy_thread(temp_buffer, ctx.buffer, 64);
            sha256_transform(ctx, temp_buffer);
            ctx.count[0] = 0;
            ++ctx.count[1];
        }
    }
}

void sha256_final(thread SHA256_CTX& ctx, thread uint8_t* hash) {
    thread uint8_t temp_buffer[64];
    uint32_t i = ctx.count[0];
    ctx.buffer[i++] = 0x80;
    
    if (i > 56) {
        while (i < 64)
            ctx.buffer[i++] = 0x00;
        metal_memcpy_thread(temp_buffer, ctx.buffer, 64);
        sha256_transform(ctx, temp_buffer);
        i = 0;
    }
    
    while (i < 56)
        ctx.buffer[i++] = 0x00;
    
    uint64_t bits = (ctx.count[1] * 64 + ctx.count[0]) * 8;
    ctx.buffer[63] = bits;
    ctx.buffer[62] = bits >> 8;
    ctx.buffer[61] = bits >> 16;
    ctx.buffer[60] = bits >> 24;
    ctx.buffer[59] = bits >> 32;
    ctx.buffer[58] = bits >> 40;
    ctx.buffer[57] = bits >> 48;
    ctx.buffer[56] = bits >> 56;
    
    metal_memcpy_thread(temp_buffer, ctx.buffer, 64);
    sha256_transform(ctx, temp_buffer);
    
    for (i = 0; i < 8; i++) {
        hash[i * 4] = (ctx.state[i] >> 24);
        hash[i * 4 + 1] = (ctx.state[i] >> 16);
        hash[i * 4 + 2] = (ctx.state[i] >> 8);
        hash[i * 4 + 3] = ctx.state[i];
    }
}

void base58_encode_32(thread const uint8_t* input, thread uint8_t* output, bool case_insensitive) {
    uint8_t digits[44] = {0};
    int digitslen = 1;
    
    for (int i = 0; i < 32; i++) {
        uint32_t carry = input[i];
        for (int j = 0; j < digitslen; j++) {
            carry += (uint32_t)(digits[j]) << 8;
            digits[j] = carry % 58;
            carry /= 58;
        }
        while (carry > 0) {
            digits[digitslen++] = carry % 58;
            carry /= 58;
        }
    }
    
    int outputlen = 0;
    for (int i = digitslen - 1; i >= 0; i--) {
        output[outputlen++] = alphanumeric[digits[i]];
    }
    while (outputlen < 44) {
        output[outputlen++] = alphanumeric[0];
    }
}

inline char to_lowercase(char c) {
    return (c >= 'A' && c <= 'Z') ? (c + 32) : c;
}

bool matches_target(thread const uint8_t* a, constant char* target, uint64_t n) {
    for (uint64_t i = 0; i < n; i++) {
        if (d_case_insensitive) {
            char a_char = to_lowercase(a[i]);
            char t_char = to_lowercase(target[i]);
            if (a_char != t_char) return false;
        } else {
            if (a[i] != target[i]) return false;
        }
    }
    return true;
}

kernel void vanity_search(
    constant uint8_t* seed [[buffer(0)]],
    constant uint8_t* base [[buffer(1)]],
    constant uint8_t* owner [[buffer(2)]],
    constant char* target [[buffer(3)]],
    constant uint64_t& target_len [[buffer(4)]],
    device uint8_t* out [[buffer(5)]],
    device atomic_int* done [[buffer(6)]],
    device atomic_uint* count [[buffer(7)]],
    uint threadgroup_position_in_grid [[threadgroup_position_in_grid]],
    uint threads_per_threadgroup [[threads_per_threadgroup]],
    uint thread_position_in_threadgroup [[thread_position_in_threadgroup]]
) {
    // Calculate global thread ID more efficiently
    uint64_t idx = (threadgroup_position_in_grid * threads_per_threadgroup) + thread_position_in_threadgroup;
    
    thread uint8_t local_out[32] = {0};
    thread uint8_t local_encoded[44] = {0};
    thread uint64_t local_seed[4];
    thread uint8_t temp_buffer[32];

    // Initialize SHA256 context for seed
    thread SHA256_CTX ctx;
    sha256_init(ctx);
    sha256_update_constant(ctx, seed, 32);
    metal_memcpy_thread(temp_buffer, (thread const uint8_t*)&idx, 8);
    sha256_update(ctx, temp_buffer, 8);
    sha256_final(ctx, (thread uint8_t*)local_seed);

    // Initialize SHA256 context for address
    thread SHA256_CTX address_sha;
    sha256_init(address_sha);
    sha256_update_constant(address_sha, base, 32);

    // Add max iterations as a constant or parameter
    const uint64_t MAX_ITERATIONS = 1000 * 1000 * 1000;
    
    for (uint64_t iter = 0; iter < MAX_ITERATIONS; iter++) {
        // Reduce atomic checks frequency
        if (iter % 100 == 0) {  // Check less frequently
            if (atomic_load_explicit(done, memory_order_relaxed) == 1) {
                atomic_fetch_add_explicit(count, iter, memory_order_relaxed);
                return;
            }
        }

        sha256_init(ctx);
        sha256_update(ctx, (thread const uint8_t*)local_seed, 16);
        sha256_final(ctx, (thread uint8_t*)local_seed);

        thread uint32_t* indices = (thread uint32_t*)&local_seed;
        thread uint8_t create_account_seed[16] = {
            alphanumeric[indices[0] % 62],
            alphanumeric[indices[1] % 62],
            alphanumeric[indices[2] % 62],
            alphanumeric[indices[3] % 62],
            alphanumeric[indices[4] % 62],
            alphanumeric[indices[5] % 62],
            alphanumeric[indices[6] % 62],
            alphanumeric[indices[7] % 62],
            alphanumeric[(indices[0] >> 2) % 62],
            alphanumeric[(indices[1] >> 2) % 62],
            alphanumeric[(indices[2] >> 2) % 62],
            alphanumeric[(indices[3] >> 2) % 62],
            alphanumeric[(indices[4] >> 2) % 62],
            alphanumeric[(indices[5] >> 2) % 62],
            alphanumeric[(indices[6] >> 2) % 62],
            alphanumeric[(indices[7] >> 2) % 62],
        };

        // Calculate and encode public key
        thread SHA256_CTX address_sha_local = address_sha;
        sha256_update(address_sha_local, create_account_seed, 16);
        sha256_update_constant(address_sha_local, owner, 32);
        sha256_final(address_sha_local, local_out);
        base58_encode_32(local_out, local_encoded, d_case_insensitive);

        if (matches_target(local_encoded, target, target_len)) {
            if (atomic_exchange_explicit(done, 1, memory_order_relaxed) == 0) {
                for (int i = 0; i < 16; i++) {
                    out[i] = create_account_seed[i];
                }
            }
            atomic_fetch_add_explicit(count, iter + 1, memory_order_relaxed);
            return;
        }
    }

    // Add explicit termination if max iterations reached
    atomic_fetch_add_explicit(count, MAX_ITERATIONS, memory_order_relaxed);

    // Add bounds checking
    if (target_len > 44) {
        atomic_store_explicit(done, 1, memory_order_relaxed);
        return;
    }
} 