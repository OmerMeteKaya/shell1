//
// Created by mete on 27.04.2026.
//

#include <termios.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include "../include/input.h"

static void render(const char *prompt, const char *buf, int len, int pos) {
    write(STDOUT_FILENO, "\r", 1);
    write(STDOUT_FILENO, prompt, strlen(prompt));
    write(STDOUT_FILENO, buf, len);
    write(STDOUT_FILENO, "\033[K", 3);   /* erase to end of line */
    int back = len - pos;
    if (back > 0) {
        char esc[16];
        snprintf(esc, sizeof(esc), "\033[%dD", back);
        write(STDOUT_FILENO, esc, strlen(esc));
    }
}

char *read_line(const char *prompt) {
    struct termios orig_termios, raw;
    
    /* Terminal durumunu kaydet */
    if (tcgetattr(STDIN_FILENO, &orig_termios) == -1) {
        return NULL;
    }
    
    /* Raw moda geç */
    raw = orig_termios;
    raw.c_lflag &= ~(ICANON | ECHO | ISIG);
    raw.c_cc[VMIN] = 1;
    raw.c_cc[VTIME] = 0;
    if (tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw) == -1) {
        return NULL;
    }
    
    char buf[MAX_INPUT] = {0};
    int len = 0;
    int pos = 0;
    int hist_off = 0;
    char saved[MAX_INPUT] = {0};
    
    render(prompt, buf, len, pos);
    
    while (1) {
        char c;
        if (read(STDIN_FILENO, &c, 1) <= 0) {
            tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
            return NULL;
        }
        
        /* Ctrl+D */
        if (c == 4) {
            if (len == 0) {
                tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
                return NULL;
            } else {
                continue;
            }
        }
        
        /* Ctrl+C */
        if (c == 3) {
            memset(buf, 0, MAX_INPUT);
            len = 0;
            pos = 0;
            hist_off = 0;
            write(STDOUT_FILENO, "\n", 1);
            render(prompt, buf, len, pos);
            continue;
        }
        
        /* Enter */
        if (c == '\r' || c == '\n') {
            buf[len] = '\0';
            tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
            write(STDOUT_FILENO, "\r\n", 2);
            if (len > 0) {
                history_add(buf);
            }
            return strdup(buf);
        }
        
        /* Backspace */
        if (c == 127 || c == 8) {
            if (pos > 0) {
                memmove(buf + pos - 1, buf + pos, len - pos);
                len--;
                pos--;
            }
            render(prompt, buf, len, pos);
            continue;
        }
        
        /* Ctrl+A */
        if (c == 1) {
            pos = 0;
            render(prompt, buf, len, pos);
            continue;
        }
        
        /* Ctrl+E */
        if (c == 5) {
            pos = len;
            render(prompt, buf, len, pos);
            continue;
        }
        
        /* Ctrl+F */
        if (c == 6) {
            if (pos < len) {
                pos++;
                render(prompt, buf, len, pos);
            }
            continue;
        }
        
        /* Ctrl+B */
        if (c == 2) {
            if (pos > 0) {
                pos--;
                render(prompt, buf, len, pos);
            }
            continue;
        }
        
        /* ESC sequence */
        if (c == 27) {
            char seq[2];
            if (read(STDIN_FILENO, &seq[0], 1) <= 0) continue;
            if (read(STDIN_FILENO, &seq[1], 1) <= 0) continue;
            
            if (seq[0] == '[') {
                /* Yukarı ok */
                if (seq[1] == 'A') {
                    if (hist_off == 0) {
                        strncpy(saved, buf, MAX_INPUT - 1);
                        saved[MAX_INPUT - 1] = '\0';
                    }
                    hist_off++;
                    char *h = history_get(hist_off);
                    if (h) {
                        strncpy(buf, h, MAX_INPUT - 1);
                        buf[MAX_INPUT - 1] = '\0';
                        len = strlen(buf);
                        pos = len;
                        free(h);
                    } else {
                        hist_off--;
                    }
                    render(prompt, buf, len, pos);
                    continue;
                }
                
                /* Aşağı ok */
                if (seq[1] == 'B') {
                    if (hist_off == 0) {
                        continue;
                    }
                    hist_off--;
                    if (hist_off == 0) {
                        strncpy(buf, saved, MAX_INPUT - 1);
                        buf[MAX_INPUT - 1] = '\0';
                        len = strlen(buf);
                        pos = len;
                    } else {
                        char *h = history_get(hist_off);
                        if (h) {
                            strncpy(buf, h, MAX_INPUT - 1);
                            buf[MAX_INPUT - 1] = '\0';
                            len = strlen(buf);
                            pos = len;
                            free(h);
                        }
                    }
                    render(prompt, buf, len, pos);
                    continue;
                }
                
                /* Sağ ok */
                if (seq[1] == 'C') {
                    if (pos < len) {
                        pos++;
                        render(prompt, buf, len, pos);
                    }
                    continue;
                }
                
                /* Sol ok */
                if (seq[1] == 'D') {
                    if (pos > 0) {
                        pos--;
                        render(prompt, buf, len, pos);
                    }
                    continue;
                }
            }
            continue;
        }
        
        /* Yazdırılabilir karakterler */
        if (c >= 32 && c < 127) {
            if (len < MAX_INPUT - 1) {
                memmove(buf + pos + 1, buf + pos, len - pos);
                buf[pos] = c;
                len++;
                pos++;
            }
            render(prompt, buf, len, pos);
            continue;
        }
    }
}
