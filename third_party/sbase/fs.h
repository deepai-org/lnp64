/* Minimal LNP64 declaration header for the vendored sbase command frontends. */
#ifndef SBASE_FS_H
#define SBASE_FS_H

#include <sys/stat.h>

enum {
	DIRFIRST = 1 << 0,
	SILENT = 1 << 1,
	IGNORE = 1 << 2,
	CONFIRM = 1 << 3,
};

struct recursor {
	void (*fn)(int, const char *, struct stat *, void *, struct recursor *);
	char *path;
	void *data;
	int depth;
	int maxdepth;
	int flags;
	int follow;
};

extern int cp_aflag;
extern int cp_fflag;
extern int cp_iflag;
extern int cp_pflag;
extern int cp_rflag;
extern int cp_vflag;
extern int cp_status;
extern int cp_follow;
extern int recurse_status;
extern int rm_status;

int cp(const char *src, const char *dst, int depth);
void recurse(int dirfd, const char *name, void *data, struct recursor *r);
void rm(int dirfd, const char *name, struct stat *st, void *data,
        struct recursor *r);

#endif
