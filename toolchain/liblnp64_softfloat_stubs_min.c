/* Weak stub implementations of softfloat helpers for linker satisfaction.
 * Only called if softfloat operations are attempted without the real library.
 * These stubs are never actually invoked in correct usage (where softfloat
 * library is linked), but their presence prevents linker errors.
 */

#include <stdint.h>

double __adddf3(double a, double b) __attribute__((weak));
double __subdf3(double a, double b) __attribute__((weak));
double __muldf3(double a, double b) __attribute__((weak));
double __divdf3(double a, double b) __attribute__((weak));
long long __fixdfdi(double a) __attribute__((weak));
double __floatdidf(int64_t i) __attribute__((weak));

double __adddf3(double a, double b) { return 0.0; }
double __subdf3(double a, double b) { return 0.0; }
double __muldf3(double a, double b) { return 0.0; }
double __divdf3(double a, double b) { return 0.0; }
long long __fixdfdi(double a) { return 0; }
double __floatdidf(int64_t i) { return 0.0; }
