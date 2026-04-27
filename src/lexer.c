//
// Created by mete on 23.04.2026.
//

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>

#include "../include/shell.h"

void tokens_free(Token *toks, int n) {
    if (!toks) return;
    for (int i = 0; i < n; i++) {
        if (toks[i].value) {
            free(toks[i].value);
        }
    }
    free(toks);
}


static char *strdup_range(const char *start, const char *end) {
    size_t len = end - start;
    char *result = malloc(len + 1);
    if (result) {
        memcpy(result, start, len);
        result[len] = '\0';
    }
    return result;
}

static Token *add_token(Token **tokens, int *count, int *capacity, TokenType type, char *value) {
    if (*count >= *capacity) {
        *capacity *= 2;
        Token *tmp = realloc(*tokens, *capacity * sizeof(Token));
        if (!tmp) {
            free(*tokens);
            return NULL;
        }
        *tokens = tmp;
    }
    (*tokens)[*count].type = type;
    (*tokens)[*count].value = value;
    (*count)++;
    return *tokens;
}

Token *lex(const char *input, int *ntokens) {
    Token *tokens = malloc(16 * sizeof(Token));
    if (!tokens) return NULL;
    
    int count = 0;
    int capacity = 16;
    const char *p = input;

    while (*p) {
        // Skip whitespace
        if (isspace(*p)) {
            p++;
            continue;
        }

        // Single character operators
        switch (*p) {
            case '|':
                if (!add_token(&tokens, &count, &capacity, TOK_PIPE, NULL)) return NULL;
                p++;
                continue;
            case '<':
                if (!add_token(&tokens, &count, &capacity, TOK_REDIR_IN, NULL)) return NULL;
                p++;
                continue;
            case '&':
                if (!add_token(&tokens, &count, &capacity, TOK_BG, NULL)) return NULL;
                p++;
                continue;
            case '>':
                if (*(p+1) == '>') {
                    if (!add_token(&tokens, &count, &capacity, TOK_REDIR_APP, NULL)) return NULL;
                    p += 2;
                } else {
                    if (!add_token(&tokens, &count, &capacity, TOK_REDIR_OUT, NULL)) return NULL;
                    p++;
                }
                continue;
        }

        // Quoted strings
        if (*p == '\'') {
            const char *start = ++p;
            while (*p && *p != '\'') p++;
            if (*p == '\'') {
                size_t len = p - start;
                char *value = malloc(len + 3);
                if (!value) {
                    tokens_free(tokens, count);
                    return NULL;
                }
                value[0] = '\'';
                memcpy(value + 1, start, len);
                value[len + 1] = '\'';
                value[len + 2] = '\0';
                if (!add_token(&tokens, &count, &capacity, TOK_WORD, value)) return NULL;
                p++;
            } else {
                tokens_free(tokens, count);
                return NULL; // Unterminated string
            }
            continue;
        }

        if (*p == '"') {
            const char *start = ++p;
            char *buffer = malloc(strlen(input) + 1);
            if (!buffer) {
                tokens_free(tokens, count);
                return NULL;
            }
            char *buf_p = buffer;
            while (*p && *p != '"') {
                if (*p == '\\' && *(p+1) == '"') {
                    *buf_p++ = '"';
                    p += 2;
                } else {
                    *buf_p++ = *p++;
                }
            }
            *buf_p = '\0';
            if (*p == '"') {
                char *value = strdup(buffer);
                free(buffer);
                if (!value) {
                    tokens_free(tokens, count);
                    return NULL;
                }
                if (!add_token(&tokens, &count, &capacity, TOK_WORD, value)) return NULL;
                p++;
            } else {
                free(buffer);
                tokens_free(tokens, count);
                return NULL; // Unterminated string
            }
            continue;
        }

        // Words
        const char *start = p;
        while (*p && !isspace(*p) && *p != '|' && *p != '<' && *p != '>' && *p != '&' && *p != '\'' && *p != '"') {
            p++;
        }
        if (p == start) {
            /* unrecognized character — skip it and continue */
            p++;
            continue;
        }
        char *value = strdup_range(start, p);
        if (!value) {
            tokens_free(tokens, count);
            return NULL;
        }
        if (!add_token(&tokens, &count, &capacity, TOK_WORD, value)) return NULL;
    }

    // Add EOF token
    if (!add_token(&tokens, &count, &capacity, TOK_EOF, NULL)) return NULL;

    *ntokens = count;
    return tokens;
}

