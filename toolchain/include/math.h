#ifndef LNP64_MATH_H
#define LNP64_MATH_H

/*
 * Minimal freestanding <math.h> for the LNP64 libc shim.
 *
 * This is intentionally a small subset: the first real-program ladder rung
 * (minimal_lua) needs the math symbols that the Lua core VM emits (floor,
 * fmod, pow, ldexp, frexp, fabs) plus a few helpers. It is NOT a complete or
 * fully IEEE-accurate libm; broader math/trig coverage is tracked as remaining
 * work in conformance_matrix.md.
 */

/* FP classification */
#define FP_NAN       0
#define FP_INFINITE  1
#define FP_ZERO      2
#define FP_SUBNORMAL 3
#define FP_NORMAL    4
#define fpclassify(x) __builtin_fpclassify(FP_NAN, FP_INFINITE, FP_NORMAL, FP_SUBNORMAL, FP_ZERO, (x))
#define isnan(x)    __builtin_isnan(x)
#define isinf(x)    __builtin_isinf(x)
#define isfinite(x) __builtin_isfinite(x)
#define isnormal(x) __builtin_isnormal(x)

#define M_PI    3.14159265358979323846
#define M_E     2.71828182845904523536
#define M_SQRT2 1.41421356237309504880
#define M_LN2   0.69314718055994530942
#define M_LOG2E 1.44269504088896340736

#define HUGE_VAL (__builtin_huge_val())
#define INFINITY (__builtin_inff())
#define NAN (__builtin_nanf(""))

double fabs(double x);
double floor(double x);
double ceil(double x);
double trunc(double x);
double fmod(double x, double y);
double sqrt(double x);
double frexp(double x, int *exp);
double ldexp(double x, int exp);
double exp(double x);
double log(double x);
double log2(double x);
double log10(double x);
double pow(double x, double y);
double round(double x);
long long llrint(double x);
long lrint(double x);
double sin(double x);
double cos(double x);
double tan(double x);
double asin(double x);
double acos(double x);
double atan(double x);
double atan2(double y, double x);
double hypot(double x, double y);
double sinh(double x);
double cosh(double x);
double tanh(double x);
double modf(double x, double *iptr);
double fmin(double a, double b);
double fmax(double a, double b);
double trunc(double x);
double cbrt(double x);
double exp2(double x);
double log2(double x);
double log10(double x);
double expm1(double x);
double log1p(double x);
double erf(double x);
double erfc(double x);
float  sinf(float x);
float  cosf(float x);
float  sqrtf(float x);
float  fabsf(float x);
float  floorf(float x);
float  ceilf(float x);
float  roundf(float x);
float  powf(float x, float y);
float  fmodf(float x, float y);
float  expf(float x);
float  logf(float x);

#endif /* LNP64_MATH_H */
