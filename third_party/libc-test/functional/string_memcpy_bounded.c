#include <string.h>
#include "test.h"

static void *(*volatile pmemcpy)(void *, const void *, size_t);
static void *(*volatile pmemset)(void *, int, size_t);

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
	char b[24];
	char src[16];
	void *p;
	size_t i;

	pmemcpy = memcpy;
	pmemset = memset;

	for (i = 0; i < sizeof src; i++) src[i] = (char)('a' + i);

	memset(b, '.', sizeof b);
	p = memcpy(b + 3, src, 10);
	if (p != b + 3) t_error("memcpy return %p != %p\n", p, b + 3);
	if (b[2] != '.' || b[13] != '.') t_error("memcpy clobbered guard bytes\n");
	expect_bytes("memcpy direct", b + 3, "abcdefghij", 10);

	memset(b, '.', sizeof b);
	p = pmemcpy(b + 1, src + 5, 6);
	if (p != b + 1) t_error("pmemcpy return %p != %p\n", p, b + 1);
	expect_bytes("memcpy pointer", b + 1, "fghijk", 6);

	memset(b, 0, sizeof b);
	p = memset(b + 4, 'x', 5);
	if (p != b + 4) t_error("memset return %p != %p\n", p, b + 4);
	expect_bytes("memset direct", b + 4, "xxxxx", 5);

	p = pmemset(b + 10, -1, 3);
	if (p != b + 10) t_error("pmemset return %p != %p\n", p, b + 10);
	for (i = 10; i < 13; i++) {
		if ((unsigned char)b[i] != 255)
			t_error("memset high byte at %d got %d\n", (int)i,
				(int)(unsigned char)b[i]);
	}

	b[20] = 'q';
	p = memcpy(b + 20, src, 0);
	if (p != b + 20) t_error("zero memcpy return %p != %p\n", p, b + 20);
	if (b[20] != 'q') t_error("zero memcpy wrote destination\n");
	p = pmemset(b + 20, 'z', 0);
	if (p != b + 20) t_error("zero memset return %p != %p\n", p, b + 20);
	if (b[20] != 'q') t_error("zero memset wrote destination\n");

	return t_status;
}
