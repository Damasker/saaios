#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>
#include <signal.h>

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static uint32_t le32(const uint8_t *p) { return le16(p) | ((uint32_t)le16(p+2) << 16); }
static int64_t now_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return -1;
    return (int64_t)t.tv_sec*1000 + t.tv_nsec/1000000;
}

/* 0=incomplete, -1=malformed, positive=one complete frame size. */
static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8 : 12;
    unsigned len = le16(p+4);
    if (len < min) return -1;
    return n < len ? 0 : (int)len;
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "self-test")) {
        uint8_t p[16] = {1,0,0,2,15,0,1,0,0,0,0,0,0,0,0};
        for (size_t n=0; n<15; n++) if (frame_size(p,n)) return 1;
        if (frame_size(p,15)!=15 || le32(p+6)!=1) return 1;
        p[4]=11; if (frame_size(p,15)!=-1) return 1;
        p[0]=2; p[4]=8; if (frame_size(p,15)!=8) return 1;
        p[0]=3; if (frame_size(p,15)!=-1) return 1;
        uint8_t r[16]={1,0,1,8,16,0,2,0,0,0,0,0,10,0,0,0};
        if (frame_size(r,15)!=0 || frame_size(r,16)!=16 ||
            le16(r+2)!=0x801 || le32(r+6)!=2 || le32(r+12)!=10) return 1;
        r[2]=1; r[3]=7; r[6]=3; r[12]=0; r[13]=0; r[15]=0;
        if (frame_size(r,15)!=0 || frame_size(r,16)!=16 ||
            le16(r+2)!=0x701 || le32(r+6)!=3) return 1;
        puts("PASS: framing, tokens, radio and registration fixtures"); return 0;
    }
    int radio = argc == 2 && !strcmp(argv[1], "query-radio-state");
    int registration = argc == 2 && !strcmp(argv[1], "query-data-registration");
    if (argc != 2 || (!radio && !registration && strcmp(argv[1], "query-sim-status"))) {
        fprintf(stderr,"usage: sit-sim-status self-test|query-sim-status|query-radio-state|query-data-registration\n"); return 64;
    }
    alarm(15);
    FILE *f=fopen("/sys/devices/platform/cpif/modem_state","r");
    char state[32]={0};
    if (!f) return 1;
    int got=fscanf(f,"%31s",state); fclose(f);
    if (got!=1 || strcmp(state,"ONLINE")) { puts("requires ONLINE"); return 1; }
    unsigned maj=0,min=0;
    f=fopen("/sys/class/cpif/umts_ipc0/dev","r");
    if (!f) return 1;
    got=fscanf(f,"%u:%u",&maj,&min); fclose(f);
    if (got!=2) return 1;
    int lock=open("/run/saaios-sit-status.lock",O_CREAT|O_RDWR|O_CLOEXEC,0600);
    if (lock<0 || flock(lock,LOCK_EX|LOCK_NB)) return 1;
    int fd=open("/dev/umts_ipc0",O_RDWR|O_NONBLOCK|O_CLOEXEC|O_NOFOLLOW);
    struct stat st;
    if (fd<0 || fstat(fd,&st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev)!=maj || minor(st.st_rdev)!=min) return 1;
    /* Factory BuildSimGetStatus: type 0, id 0x0200, length 12, token 1. */
    /* Factory BuildGetRadioState at 0x746a0: id 0x0801, length 12. */
    /* Factory registration builder domain 2: id 0x0701, no payload. */
    const unsigned request_id = registration ? 0x0701 : (radio ? 0x0801 : 0x0200);
    const uint32_t token = registration ? 3 : (radio ? 2 : 1);
    const uint8_t request[12]={0,0,(uint8_t)request_id,(uint8_t)(request_id>>8),12,0,(uint8_t)token,0,0,0,0,0};
    if (write(fd,request,sizeof(request)) != sizeof(request)) {
        perror("one-shot write"); return 1; /* Never resend or split. */
    }
    uint8_t buffer[65536]; size_t used=0; unsigned frames=0;
    int64_t deadline=now_ms()+10000;
    while (now_ms()<deadline && frames<128) {
        struct pollfd pfd={fd,POLLIN,0};
        int ready=poll(&pfd,1,500);
        if (ready<0) { if(errno==EINTR) continue; return 1; }
        if (!ready) continue;
        if (pfd.revents & (POLLERR|POLLHUP|POLLNVAL)) return 1;
        if (!(pfd.revents&POLLIN)) continue;
        ssize_t n=read(fd,buffer+used,sizeof(buffer)-used);
        if (n<0 && (errno==EINTR||errno==EAGAIN)) continue;
        if (n<=0) return 1;
        used+=(size_t)n;
        while (used) {
            int len=frame_size(buffer,used);
            if (len<0) { puts("malformed framing; stop"); return 1; }
            if (!len) break;
            frames++;
            if (buffer[0]==1 && le16(buffer+2)==request_id && le32(buffer+6)==token) {
                if (registration) {
                    printf("Data registration response: length=%d error_raw=%u\n",len,buffer[10]);
                    if (!buffer[10] && len>=16)
                        printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n",buffer[12],buffer[13],buffer[15]);
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len>=16 ? 0 : 1);
                }
                if (radio) {
                    printf("Radio response: length=%d error_raw=%u\n",len,buffer[10]);
                    if (!buffer[10] && len>=16) printf("radio_state_raw=%u\n",le32(buffer+12));
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len>=16 ? 0 : 1);
                }
                printf("SIM response: length=%d error_raw=%u\n",len,buffer[10]);
                if (!buffer[10] && len>=15)
                    printf("card_state_raw=%u universal_pin_raw=%u applications=%u\n",buffer[12],buffer[13],buffer[14]);
                close(fd); close(lock);
                return buffer[10] ? 2 : (len>=15 ? 0 : 1);
            }
            /* Drop unrelated events without logging private payloads. */
            used-=(size_t)len; memmove(buffer,buffer+len,used);
        }
        if (used==sizeof(buffer)) return 1;
    }
    printf("No matching %s response; observed_frames=%u\n",registration ? "registration" : (radio ? "radio" : "SIM"),frames);
    close(fd); close(lock); return 3;
}
