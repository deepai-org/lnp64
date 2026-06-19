static char lnp64_dot[] = ".";
static char lnp64_slash[] = "/";

static unsigned long lnp64_strlen(const char *s) {
  unsigned long n = 0;
  while (s[n] != 0)
    n = n + 1;
  return n;
}

char *basename(char *path) {
  unsigned long end;
  unsigned long start;

  if (!path || path[0] == 0)
    return lnp64_dot;

  end = lnp64_strlen(path);
  while (end > 1 && path[end - 1] == '/')
    end = end - 1;
  if (end == 1 && path[0] == '/')
    return lnp64_slash;

  start = end;
  while (start > 0 && path[start - 1] != '/')
    start = start - 1;
  path[end] = 0;
  return path + start;
}

char *dirname(char *path) {
  unsigned long end;
  unsigned long slash_pos;

  if (!path || path[0] == 0)
    return lnp64_dot;

  end = lnp64_strlen(path);
  while (end > 1 && path[end - 1] == '/')
    end = end - 1;
  if (end == 1 && path[0] == '/')
    return lnp64_slash;

  slash_pos = end;
  while (slash_pos > 0 && path[slash_pos - 1] != '/')
    slash_pos = slash_pos - 1;
  if (slash_pos == 0)
    return lnp64_dot;

  while (slash_pos > 1 && path[slash_pos - 2] == '/')
    slash_pos = slash_pos - 1;
  if (slash_pos == 1 && path[0] == '/') {
    path[1] = 0;
    return path;
  }

  path[slash_pos - 1] = 0;
  return path;
}
