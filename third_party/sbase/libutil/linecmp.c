/* See LICENSE file for copyright and license details. */
#include <string.h>

#include "../text.h"

int
linecmp(struct line *a, struct line *b)
{
	int res;
	size_t len;

	len = a->len < b->len ? a->len : b->len;
	res = memcmp(a->data, b->data, len);
	if (res)
		return res;
	if (a->len == b->len)
		return 0;
	return a->len > b->len ? 1 : -1;
}
