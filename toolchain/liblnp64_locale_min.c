/*
 * Minimal locale shim for the LNP64 libc. Only the C locale exists, so
 * localeconv() reports fixed C-locale conventions and setlocale() accepts the
 * C/POSIX locale and reports "C". This is enough for Lua number formatting and
 * scanning; broader locale support is tracked in conformance_matrix.md.
 */

#include <limits.h>
#include <locale.h>

static char lnp64_empty[] = "";
static char lnp64_dot[] = ".";
static char lnp64_c_name[] = "C";

static struct lconv lnp64_c_lconv = {
    lnp64_dot,    /* decimal_point */
    lnp64_empty,  /* thousands_sep */
    lnp64_empty,  /* grouping */
    lnp64_empty,  /* int_curr_symbol */
    lnp64_empty,  /* currency_symbol */
    lnp64_empty,  /* mon_decimal_point */
    lnp64_empty,  /* mon_thousands_sep */
    lnp64_empty,  /* mon_grouping */
    lnp64_empty,  /* positive_sign */
    lnp64_empty,  /* negative_sign */
    CHAR_MAX,     /* int_frac_digits */
    CHAR_MAX,     /* frac_digits */
    CHAR_MAX,     /* p_cs_precedes */
    CHAR_MAX,     /* p_sep_by_space */
    CHAR_MAX,     /* n_cs_precedes */
    CHAR_MAX,     /* n_sep_by_space */
    CHAR_MAX,     /* p_sign_posn */
    CHAR_MAX,     /* n_sign_posn */
};

struct lconv *localeconv(void) { return &lnp64_c_lconv; }

static int lnp64_str_eq(const char *a, const char *b) {
  while (*a && *a == *b) {
    a++;
    b++;
  }
  return *a == *b;
}

char *setlocale(int category, const char *locale) {
  (void)category;
  if (locale == 0 || locale[0] == '\0' || lnp64_str_eq(locale, "C") ||
      lnp64_str_eq(locale, "POSIX"))
    return lnp64_c_name;
  return 0; /* unsupported locale */
}
