/*
 * Minimal freestanding libm shim for the LNP64 libc.
 *
 * Provides the math symbols the Lua core VM references at link time
 * (floor, fmod, pow, ldexp, frexp, fabs) plus the helpers (sqrt, exp, log,
 * ...) that pow depends on. Exact integer-boundary operations (fabs/floor/
 * ceil/trunc/frexp/ldexp) are bit-accurate; the transcendental helpers are
 * range-reduced approximations adequate for bring-up but not certified
 * IEEE-accurate. Broader/accurate libm coverage is tracked in
 * conformance_matrix.md.
 */

#include <math.h>
#include <stdint.h>

typedef union {
  double d;
  uint64_t u;
} lnp64_dbits;

static int lnp64_exponent(uint64_t u) {
  return (int)((u >> 52) & 0x7ff);
}

double fabs(double x) {
  lnp64_dbits b;
  b.d = x;
  b.u &= 0x7fffffffffffffffULL;
  return b.d;
}

double floor(double x) {
  lnp64_dbits b;
  b.d = x;
  int e = lnp64_exponent(b.u) - 1023;
  if (e >= 52)
    return x; /* already integral, or inf/nan */
  if (e < 0) {
    if (x == 0.0)
      return x; /* preserves signed zero */
    return (b.u & 0x8000000000000000ULL) ? -1.0 : 0.0;
  }
  uint64_t mask = (1ULL << (52 - e)) - 1;
  if ((b.u & mask) == 0)
    return x; /* no fractional bits */
  if (b.u & 0x8000000000000000ULL)
    b.u += (1ULL << (52 - e)); /* round toward -inf */
  b.u &= ~mask;
  return b.d;
}

double ceil(double x) { return -floor(-x); }

double trunc(double x) {
  lnp64_dbits b;
  b.d = x;
  int e = lnp64_exponent(b.u) - 1023;
  if (e >= 52)
    return x;
  if (e < 0) {
    b.u &= 0x8000000000000000ULL; /* +/- 0 */
    return b.d;
  }
  uint64_t mask = (1ULL << (52 - e)) - 1;
  b.u &= ~mask;
  return b.d;
}

double fmod(double x, double y) {
  if (y != y || x != x)
    return x + y; /* propagate nan */
  double ay = fabs(y);
  if (ay == 0.0)
    return NAN;
  double ax = fabs(x);
  if (ax < ay)
    return x;
  if (ax == ay)
    return (x < 0.0) ? -0.0 : 0.0;
  double r = ax;
  while (r >= ay) {
    double t = ay;
    while (t + t <= r && t + t > t)
      t += t;
    r -= t;
  }
  return (x < 0.0) ? -r : r;
}

double frexp(double x, int *eptr) {
  lnp64_dbits b;
  b.d = x;
  int e = lnp64_exponent(b.u);
  if (e == 0) {
    if (x == 0.0) {
      *eptr = 0;
      return x;
    }
    b.d = x * 18014398509481984.0; /* 2^54: normalize subnormal */
    e = lnp64_exponent(b.u) - 54;
  }
  if (lnp64_exponent(b.u) == 0x7ff) {
    *eptr = 0; /* inf/nan */
    return x;
  }
  *eptr = e - 1022;
  b.u = (b.u & ~(0x7ffULL << 52)) | (1022ULL << 52);
  return b.d;
}

double ldexp(double x, int n) {
  if (n > 1023) {
    x *= 8.98846567431158e307; /* 2^1023 */
    n -= 1023;
    if (n > 1023) {
      x *= 8.98846567431158e307;
      n -= 1023;
      if (n > 1023)
        n = 1023;
    }
  } else if (n < -1022) {
    x *= 2.2250738585072014e-308; /* 2^-1022 */
    n += 1022;
    if (n < -1022) {
      x *= 2.2250738585072014e-308;
      n += 1022;
      if (n < -1022)
        n = -1022;
    }
  }
  lnp64_dbits b;
  b.u = ((uint64_t)(n + 1023)) << 52;
  return x * b.d;
}

double sqrt(double x) {
  if (x < 0.0)
    return NAN;
  if (x == 0.0 || x != x)
    return x;
  if (x == HUGE_VAL)
    return x;
  /* exponent-halving initial guess, then Newton iteration */
  int e;
  double m = frexp(x, &e);
  double g = ldexp(m + 0.5, e / 2);
  for (int i = 0; i < 12; i++)
    g = 0.5 * (g + x / g);
  return g;
}

double log(double x) {
  if (x < 0.0)
    return NAN;
  if (x == 0.0)
    return -HUGE_VAL;
  if (x != x || x == HUGE_VAL)
    return x;
  int e;
  double m = frexp(x, &e); /* m in [0.5, 1) */
  if (m < 0.70710678118654752440) {
    m *= 2.0;
    e -= 1;
  }
  /* log(m) via atanh series with s = (m-1)/(m+1) */
  double s = (m - 1.0) / (m + 1.0);
  double s2 = s * s;
  double sum = 0.0;
  double term = s;
  for (int k = 1; k < 40; k += 2) {
    sum += term / (double)k;
    term *= s2;
  }
  return 2.0 * sum + (double)e * 0.69314718055994530942; /* + e*ln2 */
}

double log2(double x) { return log(x) * 1.4426950408889634074; }

double log10(double x) { return log(x) * 0.43429448190325182765; }

double exp(double x) {
  if (x != x)
    return x;
  if (x == HUGE_VAL)
    return x;
  if (x == -HUGE_VAL)
    return 0.0;
  /* exp(x) = 2^k * exp(r), k = round(x/ln2), r in [-ln2/2, ln2/2] */
  double kf = floor(x * 1.4426950408889634074 + 0.5);
  double r = x - kf * 0.69314718055994530942;
  double term = 1.0;
  double sum = 1.0;
  for (int n = 1; n < 18; n++) {
    term *= r / (double)n;
    sum += term;
  }
  return ldexp(sum, (int)kf);
}

double pow(double x, double y) {
  if (y == 0.0)
    return 1.0;
  if (x != x || y != y)
    return NAN;
  if (y == 1.0)
    return x;
  /* exact path for integer exponents */
  if (y == floor(y) && fabs(y) < 1e18) {
    long long n = (long long)y;
    int neg = n < 0;
    unsigned long long m = neg ? (unsigned long long)(-n) : (unsigned long long)n;
    double r = 1.0;
    double base = x;
    while (m) {
      if (m & 1ULL)
        r *= base;
      base *= base;
      m >>= 1;
    }
    return neg ? 1.0 / r : r;
  }
  if (x < 0.0)
    return NAN;
  if (x == 0.0)
    return (y > 0.0) ? 0.0 : HUGE_VAL;
  return exp(y * log(x));
}

static double lnp64_sin_taylor(double x) {
  double term = x, sum = x;
  for (int n = 1; n < 20; n++) {
    term *= -x * x / ((2.0 * n) * (2.0 * n + 1.0));
    sum += term;
  }
  return sum;
}

double sin(double x) {
  if (x != x || x == HUGE_VAL || x == -HUGE_VAL)
    return NAN;
  double twopi = 6.283185307179586;
  double pi = 3.141592653589793;
  double xr = x;
  while (xr > pi)
    xr -= twopi;
  while (xr < -pi)
    xr += twopi;
  return lnp64_sin_taylor(xr);
}

double cos(double x) {
  return sin(x + 1.5707963267948966);
}

double tan(double x) {
  double c = cos(x);
  if (c == 0.0)
    return HUGE_VAL;
  return sin(x) / c;
}

double atan(double x) {
  if (x == 0.0)
    return x;
  if (fabs(x) <= 1.0) {
    double term = x, sum = x, x2 = x * x;
    for (int n = 1; n < 40; n++) {
      term *= -x2 * (2.0 * n - 1.0) / (2.0 * n + 1.0);
      sum += term;
    }
    return sum;
  }
  double sign = (x < 0.0) ? -1.0 : 1.0;
  return sign * (1.5707963267948966 - atan(1.0 / x));
}

double atan2(double y, double x) {
  if (y == 0.0 && x == 0.0)
    return 0.0;
  if (x > 0.0)
    return atan(y / x);
  if (x < 0.0) {
    double base = atan(y / x);
    return (y >= 0.0) ? base + 3.141592653589793 : base - 3.141592653589793;
  }
  return (y > 0.0) ? 1.5707963267948966 : -1.5707963267948966;
}

double asin(double x) {
  if (x < -1.0 || x > 1.0)
    return NAN;
  if (x == 0.0)
    return x;
  return atan2(x, sqrt(1.0 - x * x));
}

double acos(double x) {
  if (x < -1.0 || x > 1.0)
    return NAN;
  return 1.5707963267948966 - asin(x);
}

long long llrint(double x) {
  return (long long)(x + (x >= 0.0 ? 0.5 : -0.5));
}

long lrint(double x) {
  return (long)(x + (x >= 0.0 ? 0.5 : -0.5));
}

double round(double x) {
  return x >= 0.0 ? floor(x + 0.5) : ceil(x - 0.5);
}

double sinh(double x) {
  double e = exp(x);
  double ne = exp(-x);
  return (e - ne) * 0.5;
}
double cosh(double x) {
  double e = exp(x);
  double ne = exp(-x);
  return (e + ne) * 0.5;
}
double tanh(double x) {
  double e2x = exp(2.0 * x);
  return (e2x - 1.0) / (e2x + 1.0);
}
double modf(double x, double *iptr) {
  double i = trunc(x);
  if (iptr) *iptr = i;
  return x - i;
}
double fmin(double a, double b) { return a < b ? a : b; }
double fmax(double a, double b) { return a > b ? a : b; }
double cbrt(double x) { return x >= 0 ? pow(x, 1.0/3.0) : -pow(-x, 1.0/3.0); }
double exp2(double x) { return pow(2.0, x); }
double expm1(double x) { return exp(x) - 1.0; }
double log1p(double x) { return log(1.0 + x); }
double erf(double x) {
  /* Abramowitz & Stegun approximation */
  double t = 1.0 / (1.0 + 0.3275911 * (x < 0 ? -x : x));
  double p = ((((1.061405429*t - 1.453152027)*t) + 1.421413741)*t
              - 0.284496736)*t + 0.254829592;
  double r = 1.0 - p * t * exp(-(x*x));
  return x < 0 ? -r : r;
}
double erfc(double x) { return 1.0 - erf(x); }
double hypot(double x, double y) { return sqrt(x*x + y*y); }

/* float variants */
float sinf(float x)  { return (float)sin((double)x); }
float cosf(float x)  { return (float)cos((double)x); }
float sqrtf(float x) { return (float)sqrt((double)x); }
float fabsf(float x) { return x < 0 ? -x : x; }
float floorf(float x){ return (float)floor((double)x); }
float ceilf(float x) { return (float)ceil((double)x); }
float roundf(float x){ return (float)round((double)x); }
float powf(float x, float y) { return (float)pow((double)x,(double)y); }
float fmodf(float x, float y){ return (float)fmod((double)x,(double)y); }
float expf(float x)  { return (float)exp((double)x); }
float logf(float x)  { return (float)log((double)x); }
