#ifndef LNP64_PTHREAD_H
#define LNP64_PTHREAD_H

#include <stddef.h>

typedef unsigned long pthread_t;
typedef unsigned int pthread_key_t;
typedef struct {
  unsigned long __opaque;
} pthread_attr_t;

int pthread_create(pthread_t *thread, const pthread_attr_t *attr,
                   void *(*start_routine)(void *), void *arg);
int pthread_join(pthread_t thread, void **retval);
int pthread_detach(pthread_t thread);
pthread_t pthread_self(void);
void pthread_exit(void *retval);
int pthread_atfork(void (*prepare)(void), void (*parent)(void),
                   void (*child)(void));

int pthread_key_create(pthread_key_t *key, void (*destructor)(void *));
int pthread_key_delete(pthread_key_t key);
void *pthread_getspecific(pthread_key_t key);
int pthread_setspecific(pthread_key_t key, const void *value);

#endif
