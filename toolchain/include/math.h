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

#endif /* LNP64_MATH_H */
