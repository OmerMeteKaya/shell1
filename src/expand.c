//
// Created by mete on 26.04.2026.
//

#include "../include/shell.h"
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <ctype.h>
#include <stdio.h>
#include <glob.h>
#include <sys/wait.h>
#include <fcntl.h>
#include "../include/rc.h"

static char *itoa(int value) {
    static char buf[32];
    snprintf(buf, sizeof(buf), "%d", value);
    return strdup(buf);
}

static int is_var_char(int c) {
    return isalnum(c) || c == '_';
}

static int append_str(char **buf, size_t *len, size_t *capacity, const char *str) {
    if (!str) return 0;
    
    size_t str_len = strlen(str);
    if (*len + str_len >= *capacity) {
        size_t new_capacity = *capacity;
        while (new_capacity < *len + str_len + 1) {
            new_capacity *= 2;
        }
        char *tmp = realloc(*buf, new_capacity);
        if (!tmp) return -1;
        *buf = tmp;
        *capacity = new_capacity;
    }
    
    memcpy(*buf + *len, str, str_len);
    *len += str_len;
    return 0;
}

static char *run_command_substitution(const char *cmd_str) {
    /*
     * Runs cmd_str in a subshell, captures stdout, returns as string.
     * Caller must free() result.
     */
    if (!cmd_str || !*cmd_str) return strdup("");

    /* Create pipe to capture child stdout */
    int pipefd[2];
    if (pipe(pipefd) < 0) return strdup("");

    pid_t pid = fork();
    if (pid < 0) {
        close(pipefd[0]); close(pipefd[1]);
        return strdup("");
    }

    if (pid == 0) {
        /* child: redirect stdout to pipe write end */
        close(pipefd[0]);
        dup2(pipefd[1], STDOUT_FILENO);
        close(pipefd[1]);

        /* run cmd_str through shell pipeline */
        /* re-lex and execute */
        extern Token *lex(const char *input, int *ntokens);
        extern Token *glob_expand_tokens(Token *toks, int *ntokens, int last_exit);
        extern CmdList *parse_list(Token *toks, int ntokens);
        extern int execute_list(CmdList *list);
        extern void cmdlist_free(CmdList *list);
        extern void tokens_free(Token *toks, int n);
        extern int last_exit_status;

        int ntokens;
        Token *toks = lex(cmd_str, &ntokens);
        if (toks) {
            toks = glob_expand_tokens(toks, &ntokens, last_exit_status);
            if (toks) {
                CmdList *list = parse_list(toks, ntokens);
                if (list) {
                    execute_list(list);
                    cmdlist_free(list);
                }
                tokens_free(toks, ntokens);
            }
        }
        _exit(0);
    }

    /* parent: read from pipe read end */
    close(pipefd[1]);

    char *buf = malloc(4096);
    if (!buf) { close(pipefd[0]); waitpid(pid, NULL, 0); return strdup(""); }
    size_t total = 0, cap = 4096;

    ssize_t n;
    while ((n = read(pipefd[0], buf + total, cap - total - 1)) > 0) {
        total += n;
        if (total + 1 >= cap) {
            cap *= 2;
            char *tmp = realloc(buf, cap);
            if (!tmp) break;
            buf = tmp;
        }
    }
    close(pipefd[0]);
    waitpid(pid, NULL, 0);

    buf[total] = '\0';

    /* strip trailing newlines (standard shell behavior) */
    while (total > 0 && buf[total-1] == '\n') {
        buf[--total] = '\0';
    }

    return buf;
}

char *expand_word(const char *word, int last_exit_status) {
    if (!word) return NULL;
    
    size_t word_len = strlen(word);
    
    // Handle single-quoted strings - no expansion
    if (word_len >= 2 && word[0] == '\'' && word[word_len-1] == '\'') {
        return strdup(word);
    }
    
    char *buf = malloc(256);
    if (!buf) return strdup(word);
    
    size_t len = 0;
    size_t capacity = 256;
    
    const char *p = word;
    
    // Handle double-quoted strings - expand $ only, but keep quotes for processing
    int in_double_quotes = 0;
    if (word_len >= 2 && word[0] == '"' && word[word_len-1] == '"') {
        in_double_quotes = 1;
        p++; // Skip opening quote
        word_len--; // Adjust length
    }
    
    while (*p && (in_double_quotes ? (p < word + word_len) : *p != '\0')) {
        // Handle tilde expansion (only at start of word or after slash, and not in double quotes for mid-word)
        if (*p == '~' && (p == word || *(p-1) == '/') && !in_double_quotes) {
            const char *home = getenv("HOME");
            if (!home) home = "";
            
            if (append_str(&buf, &len, &capacity, home) < 0) {
                free(buf);
                return strdup(word);
            }
            p++;
            continue;
        }
        
        // Handle variable expansion
        if (*p == '$') {
            p++;
            
            // Handle "$?" - last exit status
            if (*p == '?') {
                char *status_str = itoa(last_exit_status);
                if (!status_str) {
                    if (append_str(&buf, &len, &capacity, "$?") < 0) {
                        free(buf);
                        return strdup(word);
                    }
                } else {
                    if (append_str(&buf, &len, &capacity, status_str) < 0) {
                        free(status_str);
                        free(buf);
                        return strdup(word);
                    }
                    free(status_str);
                }
                p++;
                continue;
            }
            
            // Handle "$$" - process ID
            if (*p == '$') {
                char *pid_str = itoa(getpid());
                if (!pid_str) {
                    if (append_str(&buf, &len, &capacity, "$$") < 0) {
                        free(buf);
                        return strdup(word);
                    }
                } else {
                    if (append_str(&buf, &len, &capacity, pid_str) < 0) {
                        free(pid_str);
                        free(buf);
                        return strdup(word);
                    }
                    free(pid_str);
                }
                p++;
                continue;
            }
            
            // $(...) command substitution
            if (*p == '(') {
                p++;  /* skip '(' */
                const char *cmd_start = p;
                int depth = 1;
                /* find matching closing paren, handle nesting */
                while (*p && depth > 0) {
                    if (*p == '(') depth++;
                    else if (*p == ')') depth--;
                    p++;
                }
                /* p now points after the closing ')' */
                int cmd_len = (p - 1) - cmd_start;  /* exclude closing ')' */
                char *cmd_str = strndup(cmd_start, cmd_len);
                if (cmd_str) {
                    char *output = run_command_substitution(cmd_str);
                    free(cmd_str);
                    if (output) {
                        append_str(&buf, &len, &capacity, output);
                        free(output);
                    }
                }
                continue;
            }

            // Handle "${VAR}" - bracketed variable
            if (*p == '{') {
                p++;
                const char *var_start = p;
                while (*p && *p != '}') p++;
                
                if (*p == '}') {
                    size_t var_len = p - var_start;
                    if (var_len > 0) {
                        char *var_name = strndup(var_start, var_len);
                        if (var_name) {
                            const char *var_value = getenv(var_name);
                            if (!var_value) var_value = "";
                            
                            if (append_str(&buf, &len, &capacity, var_value) < 0) {
                                free(var_name);
                                free(buf);
                                return strdup(word);
                            }
                            free(var_name);
                        }
                    }
                    p++;
                    continue;
                } else {
                    // Malformed ${ - treat literally
                    p = var_start - 1;
                }
            }
            
            // Handle "$VAR" - unbracketed variable
            if (is_var_char(*p)) {
                const char *var_start = p;
                while (is_var_char(*p)) p++;
                
                size_t var_len = p - var_start;
                if (var_len > 0) {
                    char *var_name = strndup(var_start, var_len);
                    if (var_name) {
                        const char *var_value = getenv(var_name);
                        if (!var_value) var_value = "";
                        
                        if (append_str(&buf, &len, &capacity, var_value) < 0) {
                            free(var_name);
                            free(buf);
                            return strdup(word);
                        }
                        free(var_name);
                    }
                    continue;
                }
            }
            
            // If we get here, it was just a lone '$'
            if (append_str(&buf, &len, &capacity, "$") < 0) {
                free(buf);
                return strdup(word);
            }
            continue;
        }
        
        // Handle regular characters
        if (len + 2 > capacity) {
            capacity *= 2;
            char *tmp = realloc(buf, capacity);
            if (!tmp) {
                free(buf);
                return strdup(word);
            }
            buf = tmp;
        }
        
        buf[len++] = *p++;
    }
    
    // Add null terminator
    if (len >= capacity) {
        char *tmp = realloc(buf, capacity + 1);
        if (!tmp) {
            free(buf);
            return strdup(word);
        }
        buf = tmp;
    }
    buf[len] = '\0';
    
    return buf;
}

static int has_glob_chars(const char *word) {
    return strpbrk(word, "*?[") != NULL;
}

void expand_tokens(Token *toks, int ntokens, int last_exit_status) {
    if (!toks) return;
    
    for (int i = 0; i < ntokens; i++) {
        if (toks[i].type == TOK_WORD && toks[i].value) {
            char *expanded = expand_word(toks[i].value, last_exit_status);
            if (expanded) {
                free(toks[i].value);
                toks[i].value = expanded;
            }
        }
    }
}

Token *glob_expand_tokens(Token *toks, int *ntokens, int last_exit_status) {
    if (!toks || !ntokens) return NULL;
    
    // Önce değişken genişletmesi yap
    expand_tokens(toks, *ntokens, last_exit_status);
    
    // Genişletilmiş token sayısını hesapla
    int expanded_count = 0;
    for (int i = 0; i < *ntokens; i++) {
        if (toks[i].type == TOK_WORD && toks[i].value && has_glob_chars(toks[i].value)) {
            glob_t g;
            int ret = glob(toks[i].value, GLOB_NOCHECK|GLOB_TILDE, NULL, &g);
            if (ret == 0 || ret == GLOB_NOMATCH) {
                expanded_count += g.gl_pathc;
                globfree(&g);
            } else {
                expanded_count++; // Hata durumunda orijinal token'ı koru
            }
        } else {
            expanded_count++;
        }
    }
    
    // Yeni token dizisini oluştur
    Token *new_toks = malloc((expanded_count + 1) * sizeof(Token));
    if (!new_toks) return NULL;
    
    int new_index = 0;
    for (int i = 0; i < *ntokens; i++) {
        if (toks[i].type == TOK_WORD && toks[i].value && has_glob_chars(toks[i].value)) {
            glob_t g;
            int ret = glob(toks[i].value, GLOB_NOCHECK|GLOB_TILDE, NULL, &g);
            if (ret == 0 || ret == GLOB_NOMATCH) {
                for (size_t j = 0; j < g.gl_pathc; j++) {
                    new_toks[new_index].type = TOK_WORD;
                    new_toks[new_index].value = strdup(g.gl_pathv[j]);
                    if (!new_toks[new_index].value) {
                        // Bellek hatası durumunda kaynakları temizle
                        for (int k = 0; k < new_index; k++) {
                            free(new_toks[k].value);
                        }
                        free(new_toks);
                        globfree(&g);
                        return NULL;
                    }
                    new_index++;
                }
                globfree(&g);
            } else {
                // Glob başarısız oldu, orijinal token'ı kopyala
                new_toks[new_index].type = toks[i].type;
                new_toks[new_index].value = strdup(toks[i].value);
                if (!new_toks[new_index].value) {
                    for (int k = 0; k < new_index; k++) {
                        free(new_toks[k].value);
                    }
                    free(new_toks);
                    return NULL;
                }
                new_index++;
            }
        } else {
            // Diğer token'ları kopyala
            new_toks[new_index].type = toks[i].type;
            if (toks[i].value) {
                new_toks[new_index].value = strdup(toks[i].value);
                if (!new_toks[new_index].value) {
                    for (int k = 0; k < new_index; k++) {
                        free(new_toks[k].value);
                    }
                    free(new_toks);
                    return NULL;
                }
            } else {
                new_toks[new_index].value = NULL;
            }
            new_index++;
        }
    }
    
    // EOF token'ı ekle
    new_toks[new_index].type = TOK_EOF;
    new_toks[new_index].value = NULL;
    
    // Eski token dizisini serbest bırak (değerleri değil)
    free(toks);
    
    *ntokens = expanded_count;
    return new_toks;
}
