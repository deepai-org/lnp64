/* See LICENSE file for copyright and license details. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../text.h"
#include "../util.h"

void
getlines(FILE *fp, struct linebuf *b)
{
	char *line = NULL;
	char *copy;
	size_t size = 0, linelen = 0;
	size_t idx;
	ssize_t len;

	while ((len = getline(&line, &size, fp)) > 0) {
		if (++b->nlines > b->capacity) {
			b->capacity += 512;
			b->lines = ereallocarray(b->lines, b->capacity, sizeof(*b->lines));
		}
		linelen = len;
		idx = b->nlines - 1;
		copy = emalloc(linelen + 1);
		memcpy(copy, line, linelen + 1);
		b->lines[idx].data = copy;
		b->lines[idx].len = linelen;
	}
	free(line);
	if (b->lines && b->nlines && linelen && b->lines[b->nlines - 1].data[linelen - 1] != '\n') {
		idx = b->nlines - 1;
		copy = erealloc(b->lines[idx].data, linelen + 2);
		b->lines[idx].data = copy;
		copy[linelen] = '\n';
		copy[linelen + 1] = '\0';
		b->lines[idx].len++;
	}
}
