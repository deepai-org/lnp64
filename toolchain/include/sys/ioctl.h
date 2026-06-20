#ifndef LNP64_SYS_IOCTL_H
#define LNP64_SYS_IOCTL_H

/* Minimal ioctl interface for compatibility.
 * Most ioctl operations are not supported on LNP64.
 */

#define TIOCGWINSZ 0x5413
#define TIOCSWINSZ 0x5414

struct winsize {
  unsigned short ws_row;
  unsigned short ws_col;
  unsigned short ws_xpixel;
  unsigned short ws_ypixel;
};

int ioctl(int fd, int request, ...);

#endif
