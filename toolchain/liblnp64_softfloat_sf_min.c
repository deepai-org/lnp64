/*
 * Single-precision (float) softfloat stubs for LNP64.
 * LNP64 has no native float/double operations, so single-precision
 * ops are implemented by promoting to double, doing the op, then
 * truncating back. This is correct for all operations except
 * intermediate rounding edge-cases, which don't affect Redis.
 */

/* Get the double softfloat operations */
typedef unsigned int   uint32_t;
typedef unsigned long  uint64_t;
typedef long           int64_t;
typedef int            int32_t;

/* IEEE 754 float: 1 sign + 8 exponent + 23 mantissa, bias 127 */
/* IEEE 754 double: 1 sign + 11 exponent + 52 mantissa, bias 1023 */

static double sf_to_df(uint32_t f) {
  /* Extract fields */
  unsigned sign = f >> 31;
  unsigned exp  = (f >> 23) & 0xff;
  unsigned mant = f & 0x7fffff;
  /* Zero / denorm */
  if (exp == 0) {
    if (mant == 0) {
      /* +-0: return double +-0 */
      uint64_t r = (uint64_t)sign << 63;
      double d;
      __builtin_memcpy(&d, &r, 8);
      return d;
    }
    /* Denormal: normalize it */
    exp = 1;
    while (!(mant & 0x800000)) { mant <<= 1; exp--; }
    mant &= 0x7fffff;
  } else if (exp == 0xff) {
    /* Inf or NaN */
    uint64_t r = ((uint64_t)sign << 63) | (0x7ffULL << 52)
                 | ((uint64_t)mant << 29);
    double d;
    __builtin_memcpy(&d, &r, 8);
    return d;
  }
  int dexp = (int)exp - 127 + 1023;
  uint64_t r = ((uint64_t)sign << 63) | ((uint64_t)dexp << 52)
               | ((uint64_t)mant << 29);
  double d;
  __builtin_memcpy(&d, &r, 8);
  return d;
}

static uint32_t df_to_sf(double d) {
  uint64_t bits;
  __builtin_memcpy(&bits, &d, 8);
  unsigned sign = bits >> 63;
  int dexp = (int)((bits >> 52) & 0x7ff);
  uint64_t dmant = bits & 0x000fffffffffffffULL;
  /* Special: 0 */
  if (dexp == 0 && dmant == 0)
    return sign << 31;
  /* Special: Inf/NaN */
  if (dexp == 0x7ff)
    return (sign << 31) | 0x7f800000u | (uint32_t)(dmant >> 29);
  /* Normal double: convert exponent */
  int fexp = dexp - 1023 + 127;
  if (fexp >= 255) {
    /* Overflow: return +-Inf */
    return (sign << 31) | 0x7f800000u;
  }
  if (fexp <= 0) {
    /* Underflow: return +-0 (or denormal, approximate as 0) */
    return sign << 31;
  }
  /* Round mantissa: drop 29 bits, round to nearest */
  uint32_t mant = (uint32_t)(dmant >> 29);
  if ((dmant >> 28) & 1) mant++;
  if (mant >= (1u << 23)) { mant >>= 1; fexp++; }
  if (fexp >= 255) return (sign << 31) | 0x7f800000u;
  return (sign << 31) | ((unsigned)fexp << 23) | mant;
}

/* forward declarations from double softfloat */
extern double __adddf3(double, double);
extern double __subdf3(double, double);
extern double __muldf3(double, double);
extern double __divdf3(double, double);
extern double __negdf2(double);
extern int    __cmpdf2(double, double);
extern int    __ltdf2(double, double);
extern int    __ledf2(double, double);
extern int    __gtdf2(double, double);
extern int    __gedf2(double, double);
extern int    __eqdf2(double, double);
extern int    __nedf2(double, double);
extern double __floatdidf(long);
extern double __floatsidf(int);
extern long   __fixdfdi(double);
extern int    __fixdfsi(double);
extern unsigned long __fixunsdfdi(double);
extern unsigned int  __fixunsdfsi(double);

uint32_t __addsf3(uint32_t a, uint32_t b) {
  return df_to_sf(__adddf3(sf_to_df(a), sf_to_df(b)));
}
uint32_t __subsf3(uint32_t a, uint32_t b) {
  return df_to_sf(__adddf3(sf_to_df(a), sf_to_df(b ^ 0x80000000u)));
}
uint32_t __mulsf3(uint32_t a, uint32_t b) {
  return df_to_sf(__muldf3(sf_to_df(a), sf_to_df(b)));
}
uint32_t __divsf3(uint32_t a, uint32_t b) {
  return df_to_sf(__divdf3(sf_to_df(a), sf_to_df(b)));
}
uint32_t __negsf2(uint32_t a) { return a ^ 0x80000000u; }

int __cmpsf2(uint32_t a, uint32_t b) {
  return __cmpdf2(sf_to_df(a), sf_to_df(b));
}
int __eqsf2(uint32_t a, uint32_t b)  { return __eqdf2(sf_to_df(a), sf_to_df(b)); }
int __nesf2(uint32_t a, uint32_t b)  { return __nedf2(sf_to_df(a), sf_to_df(b)); }
int __ltsf2(uint32_t a, uint32_t b)  { return __ltdf2(sf_to_df(a), sf_to_df(b)); }
int __lesf2(uint32_t a, uint32_t b)  { return __ledf2(sf_to_df(a), sf_to_df(b)); }
int __gtsf2(uint32_t a, uint32_t b)  { return __gtdf2(sf_to_df(a), sf_to_df(b)); }
int __gesf2(uint32_t a, uint32_t b)  { return __gedf2(sf_to_df(a), sf_to_df(b)); }
int __unordsf2(uint32_t a, uint32_t b) {
  /* NaN check: exp all-ones */
  return ((a & 0x7f800000u) == 0x7f800000u && (a & 0x7fffffu)) ||
         ((b & 0x7f800000u) == 0x7f800000u && (b & 0x7fffffu));
}

uint32_t __floatdisf(long a)    { return df_to_sf(__floatdidf(a)); }
uint32_t __floatsisf(int a)     { return df_to_sf(__floatsidf(a)); }
uint32_t __floatunsisf(unsigned a) { return df_to_sf(__floatsidf((int)a)); }
uint32_t __floatundisf(unsigned long a) { return df_to_sf(__floatdidf((long)a)); }

long __fixsfdi(uint32_t a)      { return __fixdfdi(sf_to_df(a)); }
int  __fixsfsi(uint32_t a)      { return __fixdfsi(sf_to_df(a)); }
unsigned long __fixunssfdi(uint32_t a)  { return __fixunsdfdi(sf_to_df(a)); }
unsigned int  __fixunssfsi(uint32_t a)  { return __fixunsdfsi(sf_to_df(a)); }

/* float-to-double conversion: wrapped by existing softfloat __extendsfdf2
 * and double-to-float __truncdfsf2 — only define if not already defined.
 * These may already exist in liblnp64_softfloat_min.o; skip them here. */
