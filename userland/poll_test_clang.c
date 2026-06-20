#include <lnp64/intrinsics.h>

#include <poll.h>
#include <sys/epoll.h>
#include <sys/event.h>
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
    struct kevent change;
    struct kevent kout;
    struct timespec ktimeout = {0, 0};
    int ep;
    int kq;

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
    ev.events = EPOLLIN;
    ev.data = read_cap + 1;
    if (epoll_ctl(ep, EPOLL_CTL_MOD, (int)read_cap, &ev) != 0)
        return 25;
    if (push_byte(write_cap, 'm') != 0)
        return 26;
    if (epoll_wait(ep, &out, 1, 0) != 1)
        return 27;
    if (out.events != EPOLLIN || out.data != read_cap + 1)
        return 28;
    if (pull_byte(read_cap, 'm') != 0)
        return 29;
    if (epoll_ctl(ep, EPOLL_CTL_DEL, (int)read_cap, 0) != 0)
        return 30;
    if (push_byte(write_cap, 'd') != 0)
        return 31;
    if (epoll_wait(ep, &out, 1, 0) != 0)
        return 32;
    if (pull_byte(read_cap, 'd') != 0)
        return 33;

    kq = kqueue();
    if (kq <= 0)
        return 16;
    change.ident = read_cap;
    change.filter = EVFILT_READ;
    change.flags = EV_ADD;
    change.fflags = 0;
    change.data = 0;
    change.udata = (void *)(unsigned long)read_cap;
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != 0)
        return 17;
    if (push_byte(write_cap, 'k') != 0)
        return 18;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 1)
        return 19;
    if (kout.ident != read_cap || kout.filter != EVFILT_READ ||
        kout.udata != (void *)(unsigned long)read_cap)
        return 20;
    if (pull_byte(read_cap, 'k') != 0)
        return 21;
    change.flags = EV_DELETE;
    change.udata = 0;
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != 0)
        return 34;
    if (push_byte(write_cap, 'q') != 0)
        return 35;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 36;
    if (pull_byte(read_cap, 'q') != 0)
        return 37;
    change.flags = EV_ADD | EV_ONESHOT;
    change.udata = (void *)(unsigned long)(read_cap + 1);
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != 0)
        return 38;
    if (push_byte(write_cap, 'o') != 0)
        return 39;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 1)
        return 40;
    if (kout.ident != read_cap || kout.filter != EVFILT_READ ||
        kout.udata != (void *)(unsigned long)(read_cap + 1))
        return 41;
    if (pull_byte(read_cap, 'o') != 0)
        return 42;
    if (push_byte(write_cap, 'x') != 0)
        return 43;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 44;
    if (pull_byte(read_cap, 'x') != 0)
        return 45;
    change.filter = 99;
    change.flags = EV_ADD;
    change.udata = 0;
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != -1)
        return 46;
    change.udata = (void *)99;
    if (kevent(kq, &change, 1, &kout, 1, &ktimeout) != 1)
        return 55;
    if (kout.filter != 99 || !(kout.flags & EV_ERROR) || kout.data != 22 ||
        kout.udata != (void *)99)
        return 56;
    change.ident = 77;
    change.filter = EVFILT_USER;
    change.flags = EV_ADD;
    change.fflags = 0;
    change.data = 0;
    change.udata = (void *)77;
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != 0)
        return 57;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 58;
    change.fflags = NOTE_TRIGGER;
    if (kevent(kq, &change, 1, &kout, 1, &ktimeout) != 1)
        return 59;
    if (kout.ident != 77 || kout.filter != EVFILT_USER ||
        kout.fflags != NOTE_TRIGGER || kout.udata != (void *)77)
        return 60;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 61;
    change.ident = read_cap;
    change.filter = EVFILT_READ;
    change.fflags = 0;
    change.flags = EV_ADD | EV_DISABLE;
    change.udata = (void *)(unsigned long)(read_cap + 2);
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != 0)
        return 62;
    if (push_byte(write_cap, 'v') != 0)
        return 63;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 64;
    change.flags = EV_ENABLE;
    if (kevent(kq, &change, 1, &kout, 1, &ktimeout) != 1)
        return 65;
    if (kout.ident != read_cap || kout.filter != EVFILT_READ ||
        kout.udata != (void *)(unsigned long)(read_cap + 2))
        return 66;
    if (pull_byte(read_cap, 'v') != 0)
        return 67;
    change.ident = write_cap;
    change.filter = EVFILT_WRITE;
    change.fflags = 0;
    change.flags = EV_ADD | EV_ONESHOT;
    change.udata = (void *)(unsigned long)write_cap;
    if (kevent(kq, &change, 1, 0, 0, &ktimeout) != 0)
        return 47;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 1)
        return 48;
    if (kout.ident != write_cap || kout.filter != EVFILT_WRITE ||
        kout.udata != (void *)(unsigned long)write_cap)
        return 49;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 50;
    if (close(kq) != 0)
        return 51;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != -1)
        return 52;
    kq = kqueue();
    if (kq <= 0)
        return 53;
    if (kevent(kq, 0, 0, &kout, 1, &ktimeout) != 0)
        return 54;

    pfd.fd = (int)read_cap;
    pfd.events = POLLIN;
    pfd.revents = 0;
    if (close((int)read_cap) != 0)
        return 22;
    if (poll(&pfd, 1, 0) != 1)
        return 23;
    if (pfd.revents != POLLNVAL)
        return 24;

    write(1, "poll_test ok\n", 13);
    return 0;
}
