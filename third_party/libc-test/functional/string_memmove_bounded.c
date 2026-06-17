#include <string.h>
#include "test.h"

static void *(*volatile pmemmove)(void *, const void *, size_t);

static void expect_bytes(const char *label, const char *got, const char *want,
	size_t n)
{
	size_t i;
	for (i = 0; i < n; i++) {
		if (got[i] != want[i]) {
			t_error("%s byte %d got %d wanted %d\n", label, (int)i,
				(int)(unsigned char)got[i], (int)(unsigned char)want[i]);
			return;
		}
	}
}

int main(void)
{
	char b[16];
	void *p;

	pmemmove = memmove;

	memset(b, 0, sizeof b);
	memcpy(b, "abcdef", 6);
	p = memmove(b + 8, b, 6);
	if (p != b + 8) t_error("non-overlap return %p != %p\n", p, b + 8);
	expect_bytes("non-overlap source", b, "abcdef", 6);
	expect_bytes("non-overlap dest", b + 8, "abcdef", 6);

	memset(b, 0, sizeof b);
	memcpy(b, "abcdef", 6);
	p = pmemmove(b + 2, b, 6);
	if (p != b + 2) t_error("backward overlap return %p != %p\n", p, b + 2);
	expect_bytes("backward overlap", b, "ababcdef", 8);

	p = memmove(b, b + 2, 6);
	if (p != b) t_error("forward overlap return %p != %p\n", p, b);
	expect_bytes("forward overlap", b, "abcdefef", 8);

	b[8] = 'x';
	p = pmemmove(b + 1, b + 4, 0);
	if (p != b + 1) t_error("zero length return %p != %p\n", p, b + 1);
	if (b[8] != 'x') t_error("zero length memmove wrote destination\n");

	return t_status;
}
