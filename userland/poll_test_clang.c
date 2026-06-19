#include "lnp64_intrinsics.h"

#include <poll.h>
#include <sys/epoll.h>
#include <sys/select.h>
#include <unistd.h>

enum {
    LNP64_OBJECT_KIND_QUEUE = 2,
    LNP64_OBJECT_PROFILE_PIPE = 1,
};

static void fd_zero(fd_set *set) {
    for (int i = 0; i < 16; i = i + 1)
        set->bits[i] = 0;
}

static void fd_set_one(int fd, fd_set *set) {
    unsigned long word = (unsigned long)fd / 64;
    unsigned long bit = (unsigned long)fd % 64;
    set->bits[word] |= 1UL << bit;
}

static int fd_is_set(int fd, const fd_set *set) {
    unsigned long word = (unsigned long)fd / 64;
    unsigned long bit = (unsigned long)fd % 64;
    return (set->bits[word] & (1UL << bit)) != 0;
}

static int queue_create(lnp64_cap_t *read_cap, lnp64_cap_t *write_cap) {
    lnp64_word_t record[9];
    record[0] = LNP64_OBJECT_CTL_CREATE;
    record[1] = LNP64_OBJECT_KIND_QUEUE;
    record[2] = LNP64_OBJECT_PROFILE_PIPE;
    record[3] = 0;
    record[4] = 0;
    record[5] = 0;
    record[6] = 0;
    record[7] = 0;
    record[8] = 0;
    if (__lnp_object_ctl((lnp64_word_t)record) != 0)
        return -1;
    *read_cap = record[3];
    *write_cap = record[4];
    return 0;
}

static int push_byte(lnp64_cap_t cap, char value) {
    return __lnp_push(cap, (lnp64_word_t)&value, 1) == 1 ? 0 : -1;
}

static int pull_byte(lnp64_cap_t cap, char expected) {
    char value = 0;
    if (__lnp_pull(cap, (lnp64_word_t)&value, 1) != 1)
        return -1;
    return value == expected ? 0 : -1;
}

int main(void) {
    lnp64_cap_t read_cap;
    lnp64_cap_t write_cap;
    struct pollfd pfd;
    fd_set readfds;
    struct timeval timeout = {0, 0};
    struct epoll_event ev;
    struct epoll_event out;
    int ep;

    if (queue_create(&read_cap, &write_cap) != 0)
        return 1;

    pfd.fd = (int)read_cap;
    pfd.events = POLLIN;
    pfd.revents = 0;
    if (push_byte(write_cap, 'p') != 0)
        return 2;
    if (poll(&pfd, 1, 0) != 1)
        return 3;
    if (pfd.revents != POLLIN)
        return 4;
    if (pull_byte(read_cap, 'p') != 0)
        return 5;

    fd_zero(&readfds);
    fd_set_one((int)read_cap, &readfds);
    if (push_byte(write_cap, 's') != 0)
        return 6;
    if (select((int)read_cap + 1, &readfds, 0, 0, &timeout) != 1)
        return 7;
    if (!fd_is_set((int)read_cap, &readfds))
        return 8;
    if (pull_byte(read_cap, 's') != 0)
        return 9;

    ep = epoll_create1(0);
    if (ep <= 0)
        return 10;
    ev.events = EPOLLIN;
    ev.data = read_cap;
    if (epoll_ctl(ep, EPOLL_CTL_ADD, (int)read_cap, &ev) != 0)
        return 11;
    if (push_byte(write_cap, 'e') != 0)
        return 12;
    if (epoll_wait(ep, &out, 1, 0) != 1)
        return 13;
    if (out.events != EPOLLIN || out.data != read_cap)
        return 14;
    if (pull_byte(read_cap, 'e') != 0)
        return 15;

    pfd.fd = (int)read_cap;
    pfd.events = POLLIN;
    pfd.revents = 0;
    if (close((int)read_cap) != 0)
        return 16;
    if (poll(&pfd, 1, 0) != 1)
        return 17;
    if (pfd.revents != POLLNVAL)
        return 18;

    write(1, "poll_test ok\n", 13);
    return 0;
}
