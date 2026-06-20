/*
 * Minimal soft-float runtime for the LNP64 libc.
 *
 * The LNP64 backend has no hardware floating point, so Clang lowers double/
 * float operations to these compiler-rt/libgcc-style helper symbols. We provide
 * IEEE-754 round-to-nearest-even implementations for the double-precision
 * arithmetic, comparison, and conversion helpers the toolchain emits (enough to
 * link and run C that uses `double`, e.g. Lua's lua_Number). Single-precision is
 * supported only through the convert helpers (__truncdfsf2/__extendsfdf2) that
 * appear in practice. Broader/edge-case soft-float coverage is tracked in
 * conformance_matrix.md.
 */

#include <stdint.h>

typedef union {
  double d;
  uint64_t u;
} lnp64_du;

typedef union {
  float f;
  uint32_t u;
} lnp64_fu;

#define LNP64_DBL_SIGN 0x8000000000000000ULL
#define LNP64_DBL_EXP 0x7ff0000000000000ULL
#define LNP64_DBL_MANT 0x000fffffffffffffULL
#define LNP64_DBL_HIDDEN 0x0010000000000000ULL /* 2^52 */
#define LNP64_DBL_QNAN 0x7ff8000000000000ULL
#define LNP64_DBL_INF 0x7ff0000000000000ULL

static double lnp64_from_bits(uint64_t u) {
  lnp64_du v;
  v.u = u;
  return v.d;
}

static uint64_t lnp64_to_bits(double d) {
  lnp64_du v;
  v.d = d;
  return v.u;
}

/* Pack a normalized result. sign in bit63. exp is the unbiased exponent of the
 * value mant * 2^(exp), where mant is a 53-bit significand with the hidden bit
 * at position 52, plus guard/round/sticky captured in the low 3 bits below it
 * via the `extra` argument (already-OR'd sticky). Performs round-to-nearest-
 * even and handles overflow/underflow/subnormals. */
static double lnp64_pack(uint64_t sign, int exp, uint64_t mant) {
  /* mant holds the significand left-extended: bit 55 is the hidden 1, bits
   * 2..0 are guard/round/sticky. Normalize so the hidden bit sits at bit 55. */
  if (mant == 0)
    return lnp64_from_bits(sign);

  /* Normalize to put the leading 1 at bit 55. */
  while (mant < (1ULL << 55)) {
    mant <<= 1;
    exp--;
  }
  while (mant >= (1ULL << 56)) {
    uint64_t sticky = mant & 1;
    mant = (mant >> 1) | sticky;
    exp++;
  }

  /* exp is the exponent for a value with the leading bit at 2^52, so the
   * biased exponent is exp + 1023. Account for the 3 extra low bits. */
  int biased = exp + 1023;

  if (biased >= 0x7ff)
    return lnp64_from_bits(sign | LNP64_DBL_INF); /* overflow -> inf */

  if (biased <= 0) {
    /* subnormal or underflow: shift right by (1 - biased) */
    int shift = 1 - biased;
    if (shift > 56)
      return lnp64_from_bits(sign); /* underflow to zero */
    uint64_t sticky = 0;
    for (int i = 0; i < shift; i++) {
      sticky |= mant & 1;
      mant >>= 1;
    }
    mant |= sticky;
    biased = 0;
  }

  /* Round to nearest even using guard/round/sticky in low 3 bits. */
  uint64_t lowbits = mant & 0x7; /* g r s */
  uint64_t frac = mant >> 3;     /* 53-bit significand (incl hidden) */
  if (lowbits > 0x4 || (lowbits == 0x4 && (frac & 1))) {
    frac++;
    if (frac >= (1ULL << 53)) { /* carry out of significand */
      frac >>= 1;
      biased++;
      if (biased >= 0x7ff)
        return lnp64_from_bits(sign | LNP64_DBL_INF);
    }
  }

  if (biased == 0 && (frac & LNP64_DBL_HIDDEN)) {
    /* rounding pushed a subnormal up to the smallest normal */
    biased = 1;
  }

  uint64_t result = sign | ((uint64_t)biased << 52) | (frac & LNP64_DBL_MANT);
  return lnp64_from_bits(result);
}

/* Unpack into sign, unbiased exponent (of hidden-bit-at-2^52), and a 56-bit
 * significand placed with the hidden bit at bit 55 and 3 zero g/r/s bits. */
static int lnp64_unpack(uint64_t bits, uint64_t *sign, int *exp,
                        uint64_t *mant) {
  *sign = bits & LNP64_DBL_SIGN;
  int biased = (int)((bits & LNP64_DBL_EXP) >> 52);
  uint64_t frac = bits & LNP64_DBL_MANT;
  if (biased == 0x7ff)
    return frac ? 2 : 1; /* 2 = NaN, 1 = Inf */
  if (biased == 0) {
    if (frac == 0) {
      *exp = 0;
      *mant = 0;
      return 0; /* zero */
    }
    /* subnormal: value = frac * 2^(-1074) */
    *exp = -1022;
    *mant = frac << 3;
    return 3; /* finite subnormal (no hidden bit) */
  }
  *exp = biased - 1023;
  *mant = (frac | LNP64_DBL_HIDDEN) << 3;
  return 3; /* finite normal */
}

double __adddf3(double a, double b) {
  uint64_t ua = lnp64_to_bits(a), ub = lnp64_to_bits(b);
  uint64_t sa, sb;
  int ea, eb, ca, cb;
  uint64_t ma, mb;
  ca = lnp64_unpack(ua, &sa, &ea, &ma);
  cb = lnp64_unpack(ub, &sb, &eb, &mb);

  if (ca == 2 || cb == 2)
    return lnp64_from_bits(LNP64_DBL_QNAN);
  if (ca == 1 || cb == 1) {
    if (ca == 1 && cb == 1 && sa != sb)
      return lnp64_from_bits(LNP64_DBL_QNAN); /* inf - inf */
    return lnp64_from_bits(ca == 1 ? ua : ub);
  }
  if (ca == 0 && cb == 0)
    return lnp64_from_bits((ua == ub) ? ua : (sa & sb)); /* +0 unless both -0 */
  if (ca == 0)
    return b;
  if (cb == 0)
    return a;

  /* Align exponents. */
  if (ea < eb) {
    int t = ea; ea = eb; eb = t;
    uint64_t tm = ma; ma = mb; mb = tm;
    uint64_t ts = sa; sa = sb; sb = ts;
  }
  int shift = ea - eb;
  if (shift > 60) {
    mb = (mb != 0); /* collapse to sticky */
  } else if (shift > 0) {
    uint64_t sticky = 0;
    for (int i = 0; i < shift; i++) {
      sticky |= mb & 1;
      mb >>= 1;
    }
    mb |= sticky;
  }

  uint64_t sign;
  uint64_t mant;
  if (sa == sb) {
    sign = sa;
    mant = ma + mb;
  } else {
    sign = sa;
    if (ma >= mb) {
      mant = ma - mb;
    } else {
      mant = mb - ma;
      sign = sb;
    }
    if (mant == 0)
      return lnp64_from_bits(0); /* exact cancellation -> +0 */
  }
  return lnp64_pack(sign, ea, mant);
}

double __subdf3(double a, double b) {
  uint64_t ub = lnp64_to_bits(b);
  return __adddf3(a, lnp64_from_bits(ub ^ LNP64_DBL_SIGN));
}

double __muldf3(double a, double b) {
  uint64_t ua = lnp64_to_bits(a), ub = lnp64_to_bits(b);
  uint64_t sa, sb;
  int ea, eb, ca, cb;
  uint64_t ma, mb;
  ca = lnp64_unpack(ua, &sa, &ea, &ma);
  cb = lnp64_unpack(ub, &sb, &eb, &mb);
  uint64_t sign = sa ^ sb;

  if (ca == 2 || cb == 2)
    return lnp64_from_bits(LNP64_DBL_QNAN);
  if (ca == 1 || cb == 1) {
    if (ca == 0 || cb == 0)
      return lnp64_from_bits(LNP64_DBL_QNAN); /* inf * 0 */
    return lnp64_from_bits(sign | LNP64_DBL_INF);
  }
  if (ca == 0 || cb == 0)
    return lnp64_from_bits(sign);

  /* Significands are 56-bit (hidden at bit55, 3 g/r/s zero). Strip the 3 low
   * zero bits to get 53-bit values, multiply to 106 bits, then reduce. */
  uint64_t xa = ma >> 3; /* 53-bit */
  uint64_t xb = mb >> 3; /* 53-bit */

  /* 64x64 -> 128 via 32-bit limbs. */
  uint64_t al = xa & 0xffffffffULL, ah = xa >> 32;
  uint64_t bl = xb & 0xffffffffULL, bh = xb >> 32;
  uint64_t ll = al * bl;
  uint64_t lh = al * bh;
  uint64_t hl = ah * bl;
  uint64_t hh = ah * bh;
  uint64_t mid = (ll >> 32) + (lh & 0xffffffffULL) + (hl & 0xffffffffULL);
  uint64_t lo = (ll & 0xffffffffULL) | (mid << 32);
  uint64_t hi = hh + (lh >> 32) + (hl >> 32) + (mid >> 32);

  /* Product occupies up to 106 bits (hi:lo). Leading bit at 2^104 or 2^105.
   * We want a 56-bit significand (hidden at bit55) with g/r/s. Combine the
   * exponent: result value = product * 2^(ea+eb-104). */
  int exp = ea + eb;
  /* Reduce the 128-bit product to 56 bits with sticky. The full product has
   * its top set bit at bit 104 (53*53 -> up to 106 bits => indices 0..105). */
  /* Find shift to land the leading bit at position 55. */
  uint64_t mant;
  uint64_t sticky = 0;
  /* Bring high part down: we keep top 56 bits. Total significant bits live in
   * hi:lo (128). Shift right by (105-55)=50 nominally, adjust by leading bit. */
  /* Compute via iterative normalization on a 128-bit value (hi:lo). */
  /* First, shift right by 49 so the 106-bit product (max bit105) maps toward
   * bit 56 region, collecting sticky. */
  int sh = 49;
  for (int i = 0; i < sh; i++) {
    sticky |= lo & 1;
    lo = (lo >> 1) | (hi << 63);
    hi >>= 1;
  }
  mant = lo | sticky; /* now < 2^57 roughly; pack will normalize */
  /* product had factor 2^(ea+eb) with significands scaled by 2^52 each =>
   * combined hidden weight 2^104; after >>49 the leading bit sits near bit55..56
   * representing exponent base (ea+eb). */
  (void)hh;
  return lnp64_pack(sign, exp, mant);
}

double __divdf3(double a, double b) {
  uint64_t ua = lnp64_to_bits(a), ub = lnp64_to_bits(b);
  uint64_t sa, sb;
  int ea, eb, ca, cb;
  uint64_t ma, mb;
  ca = lnp64_unpack(ua, &sa, &ea, &ma);
  cb = lnp64_unpack(ub, &sb, &eb, &mb);
  uint64_t sign = sa ^ sb;

  if (ca == 2 || cb == 2)
    return lnp64_from_bits(LNP64_DBL_QNAN);
  if (ca == 1 && cb == 1)
    return lnp64_from_bits(LNP64_DBL_QNAN); /* inf/inf */
  if (ca == 1)
    return lnp64_from_bits(sign | LNP64_DBL_INF);
  if (cb == 1)
    return lnp64_from_bits(sign); /* finite/inf -> 0 */
  if (cb == 0) {
    if (ca == 0)
      return lnp64_from_bits(LNP64_DBL_QNAN); /* 0/0 */
    return lnp64_from_bits(sign | LNP64_DBL_INF); /* x/0 -> inf */
  }
  if (ca == 0)
    return lnp64_from_bits(sign);

  uint64_t na = ma >> 3; /* 53-bit */
  uint64_t nb = mb >> 3; /* 53-bit */

  /* Restoring long division. Each iteration emits one quotient bit (compare
   * and subtract the proper remainder, then shift). After K iterations
   * q ~= (na/nb) * 2^(K-1); with na,nb in [2^52,2^53) and na/nb in (0.5,2),
   * K=58 keeps q under 2^59. pack reads value = q * 2^(exp-55), so
   * exp = ea-eb + 56 - K = ea-eb-2. */
  uint64_t q = 0;
  uint64_t rem = na;
  for (int i = 0; i < 58; i++) {
    q <<= 1;
    if (rem >= nb) {
      rem -= nb;
      q |= 1;
    }
    rem <<= 1;
  }
  uint64_t sticky = (rem != 0);
  uint64_t mant = q | sticky;
  int exp = ea - eb - 2;
  return lnp64_pack(sign, exp, mant);
}

/* ---- comparisons (libgcc ABI) ---- */
static int lnp64_dcmp(double A, double B, int unord) {
  lnp64_du a, b;
  a.d = A;
  b.d = B;
  uint64_t am = a.u & ~LNP64_DBL_SIGN;
  uint64_t bm = b.u & ~LNP64_DBL_SIGN;
  if (am > LNP64_DBL_INF || bm > LNP64_DBL_INF)
    return unord; /* NaN */
  if (am == 0 && bm == 0)
    return 0; /* +0 == -0 */
  if (a.u == b.u)
    return 0;
  int as = (int)(a.u >> 63), bs = (int)(b.u >> 63);
  if (as != bs)
    return as ? -1 : 1;
  if (!as)
    return am < bm ? -1 : 1;
  return am > bm ? -1 : 1;
}

int __ltdf2(double a, double b) { return lnp64_dcmp(a, b, 1); }
int __ledf2(double a, double b) { return lnp64_dcmp(a, b, 1); }
int __eqdf2(double a, double b) { return lnp64_dcmp(a, b, 1); }
int __nedf2(double a, double b) { return lnp64_dcmp(a, b, 1); }
int __gtdf2(double a, double b) { return lnp64_dcmp(a, b, -1); }
int __gedf2(double a, double b) { return lnp64_dcmp(a, b, -1); }
int __cmpdf2(double a, double b) { return lnp64_dcmp(a, b, 1); }

int __unorddf2(double a, double b) {
  lnp64_du x, y;
  x.d = a;
  y.d = b;
  uint64_t xm = x.u & ~LNP64_DBL_SIGN;
  uint64_t ym = y.u & ~LNP64_DBL_SIGN;
  return (xm > LNP64_DBL_INF || ym > LNP64_DBL_INF) ? 1 : 0;
}

/* ---- conversions ---- */
double __floatdidf(int64_t i) {
  if (i == 0)
    return 0.0;
  uint64_t sign = 0;
  uint64_t mag;
  if (i < 0) {
    sign = LNP64_DBL_SIGN;
    mag = (uint64_t)(-(i + 1)) + 1ULL; /* avoid overflow at INT64_MIN */
  } else {
    mag = (uint64_t)i;
  }
  /* Normalize mag so leading bit at position 55 (with 3 g/r/s low bits). */
  int msb = 63;
  while (!((mag >> msb) & 1))
    msb--;
  /* value = mag = mag, exponent of leading bit is msb. Build significand. */
  uint64_t mant;
  int exp = msb;
  if (msb >= 55) {
    int sh = msb - 55;
    uint64_t sticky = 0;
    for (int k = 0; k < sh; k++) {
      sticky |= mag & 1;
      mag >>= 1;
    }
    mant = mag | sticky;
  } else {
    mant = mag << (55 - msb);
  }
  return lnp64_pack(sign, exp, mant);
}

double __floatsidf(int32_t i) { return __floatdidf((int64_t)i); }

double __floatundidf(uint64_t mag) {
  if (mag == 0)
    return 0.0;
  int msb = 63;
  while (!((mag >> msb) & 1))
    msb--;
  uint64_t mant;
  int exp = msb;
  if (msb >= 55) {
    int sh = msb - 55;
    uint64_t sticky = 0;
    for (int k = 0; k < sh; k++) {
      sticky |= mag & 1;
      mag >>= 1;
    }
    mant = mag | sticky;
  } else {
    mant = mag << (55 - msb);
  }
  return lnp64_pack(0, exp, mant);
}

double __floatunsidf(uint32_t i) { return __floatundidf((uint64_t)i); }

int64_t __fixdfdi(double a) {
  uint64_t sign;
  int exp, cls;
  uint64_t mant;
  cls = lnp64_unpack(lnp64_to_bits(a), &sign, &exp, &mant);
  if (cls == 0)
    return 0;
  if (cls == 1 || cls == 2) /* inf/nan */
    return sign ? INT64_MIN : INT64_MAX;
  uint64_t frac = mant >> 3; /* 53-bit incl hidden, value = frac * 2^(exp-52) */
  int e = exp - 52;
  uint64_t result;
  if (e >= 0) {
    if (exp >= 63)
      return sign ? INT64_MIN : INT64_MAX; /* out of range */
    result = frac << e;
  } else {
    if (-e >= 64)
      return 0;
    result = frac >> (-e); /* truncate toward zero */
  }
  return sign ? -(int64_t)result : (int64_t)result;
}

int32_t __fixdfsi(double a) {
  int64_t v = __fixdfdi(a);
  if (v > INT32_MAX)
    return INT32_MAX;
  if (v < INT32_MIN)
    return INT32_MIN;
  return (int32_t)v;
}

uint64_t __fixunsdfdi(double a) {
  uint64_t sign;
  int exp, cls;
  uint64_t mant;
  cls = lnp64_unpack(lnp64_to_bits(a), &sign, &exp, &mant);
  if (cls == 0 || sign)
    return 0;
  if (cls == 1 || cls == 2)
    return UINT64_MAX;
  uint64_t frac = mant >> 3;
  int e = exp - 52;
  if (e >= 0) {
    if (exp >= 64)
      return UINT64_MAX;
    return frac << e;
  }
  if (-e >= 64)
    return 0;
  return frac >> (-e);
}

float __truncdfsf2(double a) {
  uint64_t sign;
  int exp, cls;
  uint64_t mant;
  cls = lnp64_unpack(lnp64_to_bits(a), &sign, &exp, &mant);
  lnp64_fu out;
  uint32_t s = (uint32_t)(sign >> 32) & 0x80000000U;
  if (cls == 2) {
    out.u = s | 0x7fc00000U;
    return out.f;
  }
  if (cls == 1) {
    out.u = s | 0x7f800000U;
    return out.f;
  }
  if (cls == 0) {
    out.u = s;
    return out.f;
  }
  /* frac (53-bit incl hidden), value = frac * 2^(exp-52). The float significand
   * is 24 bits (hidden + 23). Keep 3 guard/round/sticky bits below it for
   * round-to-nearest-even, shifting an extra (1-fb) for subnormal results. */
  uint64_t frac = mant >> 3;
  int fb = exp + 127;
  if (fb >= 0xff) {
    out.u = s | 0x7f800000U; /* overflow -> inf */
    return out.f;
  }
  int rsh = 26; /* drop 29 low bits but retain 3 as g/r/s (53-26 = 27 bits) */
  if (fb <= 0) {
    rsh += (1 - fb);
    fb = 0;
  }
  if (rsh >= 64) {
    out.u = s; /* far below smallest subnormal -> +/-0 */
    return out.f;
  }
  uint64_t sticky = 0, m = frac;
  for (int i = 0; i < rsh; i++) {
    sticky |= m & 1;
    m >>= 1;
  }
  m |= sticky;
  uint32_t grs = (uint32_t)(m & 7);
  uint32_t sig = (uint32_t)(m >> 3);
  if (grs > 4 || (grs == 4 && (sig & 1)))
    sig++;
  if (fb == 0) {
    if (sig & 0x00800000U) { /* rounded up to smallest normal */
      out.u = s | (1U << 23) | (sig & 0x007fffffU);
      return out.f;
    }
    out.u = s | sig; /* subnormal */
    return out.f;
  }
  if (sig & 0x01000000U) { /* significand carry */
    sig >>= 1;
    fb++;
  }
  if (fb >= 0xff) {
    out.u = s | 0x7f800000U;
    return out.f;
  }
  out.u = s | ((uint32_t)fb << 23) | (sig & 0x007fffffU);
  return out.f;
}

double __extendsfdf2(float a) {
  lnp64_fu in;
  in.f = a;
  uint64_t sign = (uint64_t)(in.u & 0x80000000U) << 32;
  int biased = (int)((in.u >> 23) & 0xff);
  uint32_t frac = in.u & 0x007fffffU;
  if (biased == 0xff) {
    if (frac)
      return lnp64_from_bits(sign | LNP64_DBL_QNAN);
    return lnp64_from_bits(sign | LNP64_DBL_INF);
  }
  if (biased == 0) {
    if (frac == 0)
      return lnp64_from_bits(sign);
    /* subnormal float -> normalize */
    int e = -126;
    uint64_t m = (uint64_t)frac << 3; /* place with g/r/s */
    return lnp64_pack(sign, e - 0, m << (52 - 23));
  }
  uint64_t result = sign | ((uint64_t)(biased - 127 + 1023) << 52) |
                    ((uint64_t)frac << (52 - 23));
  return lnp64_from_bits(result);
}
