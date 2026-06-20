#include <arpa/inet.h>
#include <ctype.h>
#include <string.h>

in_addr_t inet_addr(const char *cp) {
  struct in_addr in;
  if (inet_aton(cp, &in) == 0)
    return (in_addr_t)-1;
  return in.s_addr;
}

int inet_aton(const char *cp, struct in_addr *inp) {
  if (!cp || !inp)
    return 0;

  unsigned int a = 0, b = 0, c = 0, d = 0;
  int count = 0;

  while (*cp) {
    if (isdigit((unsigned char)*cp)) {
      unsigned int val = 0;
      while (isdigit((unsigned char)*cp))
        val = val * 10 + (unsigned char)*cp++ - '0';

      if (count == 0)
        a = val;
      else if (count == 1)
        b = val;
      else if (count == 2)
        c = val;
      else if (count == 3)
        d = val;
      else
        return 0;

      count++;

      if (*cp == '.')
        cp++;
      else if (*cp == '\0')
        break;
      else
        return 0;
    } else {
      return 0;
    }
  }

  if (count != 4)
    return 0;

  if (a > 255 || b > 255 || c > 255 || d > 255)
    return 0;

  inp->s_addr = (a << 24) | (b << 16) | (c << 8) | d;
  return 1;
}

char *inet_ntoa(struct in_addr in) {
  static char buffer[16];
  unsigned char *bytes = (unsigned char *)&in.s_addr;
  /* Note: byte order may vary; this is a simplification */
  int len = 0;
  unsigned int addr = in.s_addr;
  unsigned char a = (addr >> 24) & 0xff;
  unsigned char b = (addr >> 16) & 0xff;
  unsigned char c = (addr >> 8) & 0xff;
  unsigned char d = addr & 0xff;

  buffer[len++] = '0' + a / 100;
  a %= 100;
  buffer[len++] = '0' + a / 10;
  buffer[len++] = '0' + a % 10;
  buffer[len++] = '.';

  buffer[len++] = '0' + b / 100;
  b %= 100;
  buffer[len++] = '0' + b / 10;
  buffer[len++] = '0' + b % 10;
  buffer[len++] = '.';

  buffer[len++] = '0' + c / 100;
  c %= 100;
  buffer[len++] = '0' + c / 10;
  buffer[len++] = '0' + c % 10;
  buffer[len++] = '.';

  buffer[len++] = '0' + d / 100;
  d %= 100;
  buffer[len++] = '0' + d / 10;
  buffer[len++] = '0' + d % 10;
  buffer[len] = '\0';

  return buffer;
}

const char *inet_ntop(int af, const void *src, char *dst, socklen_t size) {
  if (!src || !dst || size < 1)
    return 0;

  if (af != 2) /* AF_INET */
    return 0;

  const unsigned char *addr = (const unsigned char *)src;
  int len = 0;
  for (int i = 0; i < 4; i++) {
    if (i > 0) {
      if (len + 1 >= (int)size)
        return 0;
      dst[len++] = '.';
    }
    unsigned char byte = addr[i];
    int digits = 0;
    unsigned char tmp[3];
    if (byte == 0) {
      tmp[digits++] = '0';
    } else {
      if (byte >= 100) {
        tmp[digits++] = '0' + byte / 100;
        byte %= 100;
      }
      if (byte >= 10) {
        tmp[digits++] = '0' + byte / 10;
        byte %= 10;
      }
      tmp[digits++] = '0' + byte;
    }
    if (len + digits >= (int)size)
      return 0;
    for (int j = 0; j < digits; j++)
      dst[len++] = tmp[j];
  }
  if (len >= (int)size)
    return 0;
  dst[len] = '\0';
  return dst;
}

int inet_pton(int af, const char *src, void *dst) {
  if (!src || !dst)
    return -1;

  if (af != 2) /* AF_INET */
    return -1;

  struct in_addr in;
  if (inet_aton(src, &in) == 0)
    return 0;

  memcpy(dst, &in, sizeof(struct in_addr));
  return 1;
}
