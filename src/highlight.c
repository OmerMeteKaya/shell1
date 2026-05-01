//
// Created by mete on 2.05.2026.
//

#include "../include/highlight.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/* ANSI color codes */
#define COL_RESET       "\033[0m"
#define COL_CMD_OK      "\033[32m"      /* green  — valid command */
#define COL_CMD_ERR     "\033[31m"      /* red    — not found */
#define COL_CMD_DANGER  "\033[1;31m"    /* bold red — dangerous */
#define COL_FLAG        "\033[33m"      /* yellow — -flag */
#define COL_STRING      "\033[35m"      /* magenta — "string" or 'string' */
#define COL_PATH        "\033[36m"      /* cyan — path/file */
#define COL_OPERATOR    "\033[1;37m"    /* bold white — | > & ; */

/* Helper: is_dangerous */
static int is_dangerous(const char *cmd) {
    static const char *dangerous[] = {
        "rm", "rmdir", "dd", "mkfs", "fdisk",
        "chmod", "chown", "kill", "killall", NULL
    };
    
    for (int i = 0; dangerous[i] != NULL; i++) {
        if (strcmp(cmd, dangerous[i]) == 0) {
            return 1;
        }
    }
    return 0;
}

/* Helper: cmd_color */
static const char *cmd_color(const char *word) {
    if (is_dangerous(word)) return COL_CMD_DANGER;

    /* builtin'leri kontrol et */
    extern int is_builtin(const char *cmd);
    if (is_builtin(word)) return COL_CMD_OK;

    /* alias'ları kontrol et */
    extern char *alias_expand(const char *name);
    if (alias_expand(word)) return COL_CMD_OK;

    if (word[0] == '/' || word[0] == '.') {
        return access(word, X_OK) == 0 ? COL_CMD_OK : COL_CMD_ERR;
    }

    char *path_env = getenv("PATH");
    if (!path_env) return COL_CMD_ERR;

    char path_copy[4096];
    strncpy(path_copy, path_env, sizeof(path_copy) - 1);
    path_copy[sizeof(path_copy) - 1] = '\0';

    char *dir = strtok(path_copy, ":");
    while (dir) {
        char full[1024];
        snprintf(full, sizeof(full), "%s/%s", dir, word);
        if (access(full, X_OK) == 0) return COL_CMD_OK;
        dir = strtok(NULL, ":");
    }

    return COL_CMD_ERR;
}

/* Helper: word_color */
static const char *word_color(const char *word, int is_first_word) {
    if (is_first_word) {
        return cmd_color(word);
    }
    
    if (word[0] == '-') {
        return COL_FLAG;
    }
    
    if (word[0] == '/' || word[0] == '~' || word[0] == '.') {
        return COL_PATH;
    }
    
    /* check if it contains / — likely a path */
    if (strchr(word, '/') != NULL) {
        return COL_PATH;
    }
    
    return COL_RESET;   /* plain argument */
}

/* Main function: highlight */
char *highlight(const char *buf, int len) {
    if (!buf || len == 0) {
        return strdup("");
    }

    /* Output buffer — worst case each char gets color codes */
    size_t out_cap = len * 16 + 256;
    char *out = malloc(out_cap);
    if (!out) {
        return strdup(buf);
    }
    out[0] = '\0';
    size_t out_len = 0;

    /* Helper macro to append string to out */
    #define APPEND(s) do { \
        size_t _l = strlen(s); \
        if (out_len + _l + 1 < out_cap) { \
            memcpy(out + out_len, s, _l); \
            out_len += _l; \
            out[out_len] = '\0'; \
        } \
    } while(0)

    int i = 0;
    int is_first_word = 1;   /* next word is a command name */

    while (i < len) {
        /* Skip and copy whitespace as-is */
        if (buf[i] == ' ' || buf[i] == '\t') {
            char ws[2] = {buf[i], '\0'};
            APPEND(ws);
            i++;
            continue;
        }

        /* Operators: | > < & ; */
        if (buf[i] == '|' || buf[i] == '>' || buf[i] == '<' ||
            buf[i] == '&' || buf[i] == ';') {
            APPEND(COL_OPERATOR);
            char op[2] = {buf[i], '\0'};
            APPEND(op);
            APPEND(COL_RESET);
            /* after operator, next word is a command */
            if (buf[i] == '|' || buf[i] == ';') {
                is_first_word = 1;
            }
            i++;
            continue;
        }

        /* Quoted strings: ' or " */
        if (buf[i] == '\'' || buf[i] == '"') {
            char quote = buf[i];
            APPEND(COL_STRING);
            char qc[2] = {buf[i], '\0'};
            APPEND(qc);
            i++;
            while (i < len && buf[i] != quote) {
                char c[2] = {buf[i], '\0'};
                APPEND(c);
                i++;
            }
            if (i < len) {
                char c[2] = {buf[i], '\0'};
                APPEND(c);
                i++;
            }
            APPEND(COL_RESET);
            continue;
        }

        /* Read a word */
        int word_start = i;
        while (i < len && buf[i] != ' ' && buf[i] != '\t' &&
               buf[i] != '|' && buf[i] != '>' && buf[i] != '<' &&
               buf[i] != '&' && buf[i] != ';' &&
               buf[i] != '\'' && buf[i] != '"') {
            i++;
        }
        int word_len = i - word_start;
        if (word_len == 0) { 
            i++; 
            continue; 
        }

        char word[1024] = {0};
        if (word_len < (int)sizeof(word)) {
            memcpy(word, buf + word_start, word_len);
        }

        const char *color = word_color(word, is_first_word);
        APPEND(color);
        APPEND(word);
        APPEND(COL_RESET);

        is_first_word = 0;
    }

    return out;
}
