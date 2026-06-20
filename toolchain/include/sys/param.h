#ifndef LNP64_SYS_PARAM_H
#define LNP64_SYS_PARAM_H

#include <limits.h>

#ifndef MAXPATHLEN
#define MAXPATHLEN PATH_MAX
#endif

#ifndef HZ
#define HZ 100
#endif

#ifndef NBBY
#define NBBY 8
#endif

#ifndef MIN
#define MIN(a,b) ((a)<(b)?(a):(b))
#endif

#ifndef MAX
#define MAX(a,b) ((a)>(b)?(a):(b))
#endif

#define howmany(x, y) (((x)+((y)-1))/(y))
#define roundup(x, y) ((((x)+((y)-1))/(y))*(y))

#endif
