#ifndef LNP64_SYS_SYSMACROS_H
#define LNP64_SYS_SYSMACROS_H

#define major(dev) ((unsigned int)(((dev) >> 8) & 0xfff))
#define minor(dev) ((unsigned int)((dev) & 0xff))
#define makedev(maj, min) ((((maj) & 0xfff) << 8) | ((min) & 0xff))

#endif
