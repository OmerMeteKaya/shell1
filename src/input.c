//
// Created by mete on 27.04.2026.
//

#include <termios.h>
#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <glob.h>
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
static int word_start(const char *buf, int pos) {
    int i = pos - 1;
    while (i > 0 && buf[i-1] != ' ') i--;
    return i;
}

static char *find_suggestion(const char *buf, int len) {
    if (len == 0) return NULL;

    /* 1. Try history first — find most recent cmd starting with buf */
    /* Call history_search_prefix(buf) — we will add this to history.c */
    char *h = history_search_prefix(buf);
    if (h) return h;

    /* 2. Try filesystem — last word completion */
    int ws = word_start(buf, len);  /* reuse existing helper */
    int wlen = len - ws;
    if (wlen == 0) return NULL;

    char pattern[MAX_INPUT];
    strncpy(pattern, buf + ws, wlen);
    pattern[wlen] = '\0';
    strncat(pattern, "*", MAX_INPUT - wlen - 1);

    glob_t g;
    if (glob(pattern, GLOB_MARK|GLOB_TILDE, NULL, &g) == 0 && g.gl_pathc > 0) {
        char *result = strdup(g.gl_pathv[0]);
        globfree(&g);
        return result;  /* caller frees */
    }
    globfree(&g);
    return NULL;
}

static void render_with_suggestion(const char *prompt, const char *buf, int len, int pos) {
    render(prompt, buf, len, pos);
    
    /* Show ghost suggestion only when cursor is at the end */
    if (pos == len) {
        char *sug = find_suggestion(buf, len);
        if (sug && strlen(sug) > (size_t)len) {
            const char *ghost = sug + len;  /* part not yet typed */
            /* print in dim gray */
            write(STDOUT_FILENO, "\033[2;37m", 7);
            write(STDOUT_FILENO, ghost, strlen(ghost));
            write(STDOUT_FILENO, "\033[0m", 4);
            /* move cursor back to correct position */
            int ghost_len = strlen(ghost);
            char esc[16];
            snprintf(esc, sizeof(esc), "\033[%dD", ghost_len);
            write(STDOUT_FILENO, esc, strlen(esc));
            free(sug);
        }
    }
}

/* Returns the common prefix length of all matches */
static int common_prefix_len(glob_t *g) {
    if (g->gl_pathc == 0) return 0;
    int len = strlen(g->gl_pathv[0]);
    for (size_t i = 1; i < g->gl_pathc; i++) {
        int j = 0;
        while (j < len && g->gl_pathv[0][j] == g->gl_pathv[i][j]) j++;
        len = j;
    }
    return len;
}

/* Finds the start of the current word under/before cursor */


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
    
    render_with_suggestion(prompt, buf, len, pos);
    
    while (1) {
        char c;
        if (read(STDIN_FILENO, &c, 1) <= 0) {
            tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
            return NULL;
        }
        
        /* TAB */
        if (c == '\t') {
            /* First try to accept ghost suggestion if exists */
            char *sug = find_suggestion(buf, len);
            if (sug) {
                int sug_len = strlen(sug);
                if (sug_len > len && sug_len < MAX_INPUT) {
                    strncpy(buf, sug, MAX_INPUT - 1);
                    len = sug_len;
                    pos = len;
                    free(sug);
                    render_with_suggestion(prompt, buf, len, pos);
                    continue;
                }
                free(sug);
            }
            
            /* Then fall through to existing glob completion */
            int ws = word_start(buf, pos);
            int wlen = pos - ws;
            char word[MAX_INPUT];
            strncpy(word, buf + ws, wlen);
            word[wlen] = '\0';
            
            strncat(word, "*", MAX_INPUT-wlen-1);
            
            glob_t g;
            int r = glob(word, GLOB_MARK|GLOB_TILDE, NULL, &g);
            
            if (r != 0 || g.gl_pathc == 0) {
                write(STDOUT_FILENO, "\a", 1);  /* bell */
                globfree(&g);
                continue;
            }
            
            if (g.gl_pathc == 1) {
                const char *match = g.gl_pathv[0];
                int match_len = strlen(match);
                int tail = len - pos;
                memmove(buf + ws + match_len, buf + pos, tail);
                memcpy(buf + ws, match, match_len);
                len = ws + match_len + tail;
                pos = ws + match_len;
                buf[len] = '\0';
            } else {
                int cp = common_prefix_len(&g);
                if (cp > wlen) {
                    const char *first = g.gl_pathv[0];
                    int tail = len - pos;
                    memmove(buf + ws + cp, buf + pos, tail);
                    memcpy(buf + ws, first, cp);
                    len = ws + cp + tail;
                    pos = ws + cp;
                    buf[len] = '\0';
                } else {
                    write(STDOUT_FILENO, "\r\n", 2);
                    for (size_t i = 0; i < g.gl_pathc; i++) {
                        write(STDOUT_FILENO, g.gl_pathv[i], strlen(g.gl_pathv[i]));
                        write(STDOUT_FILENO, "  ", 2);
                    }
                    write(STDOUT_FILENO, "\r\n", 2);
                }
            }
            
            globfree(&g);
            render_with_suggestion(prompt, buf, len, pos);
            continue;
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
            memset(saved, 0, MAX_INPUT);
            write(STDOUT_FILENO, "\n", 1);
            render_with_suggestion(prompt, buf, len, pos);
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
                if (len == 0) {
                    hist_off = 0;
                    memset(saved, 0, MAX_INPUT);
                }
            }
            render_with_suggestion(prompt, buf, len, pos);
            continue;
        }
        
        /* Ctrl+A */
        if (c == 1) {
            pos = 0;
            render_with_suggestion(prompt, buf, len, pos);
            continue;
        }
        
        /* Ctrl+E */
        if (c == 5) {
            pos = len;
            render_with_suggestion(prompt, buf, len, pos);
            continue;
        }
        
        /* Ctrl+F */
        if (c == 6) {
            if (pos < len) {
                pos++;
                render_with_suggestion(prompt, buf, len, pos);
            }
            continue;
        }
        
        /* Ctrl+B */
        if (c == 2) {
            if (pos > 0) {
                pos--;
                render_with_suggestion(prompt, buf, len, pos);
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
                    render_with_suggestion(prompt, buf, len, pos);
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
                    render_with_suggestion(prompt, buf, len, pos);
                    continue;
                }
                
                /* Sağ ok */
                if (seq[1] == 'C') {
                    if (pos < len) {
                        pos++;
                        render_with_suggestion(prompt, buf, len, pos);
                    }
                    continue;
                }
                
                /* Sol ok */
                if (seq[1] == 'D') {
                    if (pos > 0) {
                        pos--;
                        render_with_suggestion(prompt, buf, len, pos);
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
            render_with_suggestion(prompt, buf, len, pos);
            continue;
        }
    }
}
