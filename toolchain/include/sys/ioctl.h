#ifndef LNP64_SYS_IOCTL_H
#define LNP64_SYS_IOCTL_H

/* Minimal ioctl interface for compatibility.
 * Most ioctl operations are not supported on LNP64.
 */

#define TIOCGWINSZ 0
#define TIOCSWINSZ 0

int ioctl(int fd, int request, ...);

#endif
