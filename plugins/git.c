//
// Created by mete on 2.05.2026.
//
#include "../include/plugin.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static char *get_git_branch(void) {
    FILE *fp = popen("git symbolic-ref --short HEAD 2>/dev/null", "r");
    if (!fp) return NULL;

    char buf[256] = {0};
    if (!fgets(buf, sizeof(buf), fp)) {
        pclose(fp);
        return NULL;
    }
    pclose(fp);

    /* strip newline */
    buf[strcspn(buf, "\n")] = '\0';
    if (!buf[0]) return NULL;
    return strdup(buf);
}

static int get_git_dirty(void) {
    FILE *fp = popen("git status --porcelain 2>/dev/null", "r");
    if (!fp) return 0;

    char buf[4];
    int dirty = (fgets(buf, sizeof(buf), fp) != NULL);
    pclose(fp);
    return dirty;
}

static void get_git_ahead_behind(int *ahead, int *behind) {
    *ahead = 0;
    *behind = 0;

    FILE *fp = popen(
        "git rev-list --count --left-right @{upstream}...HEAD 2>/dev/null",
        "r");
    if (!fp) return;

    char buf[64] = {0};
    if (fgets(buf, sizeof(buf), fp))
        sscanf(buf, "%d\t%d", behind, ahead);
    pclose(fp);
}

static char *git_prompt_left(void) {
    char *branch = get_git_branch();
    if (!branch) return NULL;

    int dirty = get_git_dirty();
    int ahead = 0, behind = 0;
    get_git_ahead_behind(&ahead, &behind);

    char buf[256];
    const char *branch_color = dirty ? "\033[1;33m" : "\033[1;32m";

    snprintf(buf, sizeof(buf), "%s(%s%s)\033[0m",
             branch_color,
             branch,
             dirty ? "*" : "");

    if (ahead > 0 || behind > 0) {
        char extra[64];
        snprintf(extra, sizeof(extra),
                 " \033[32m↑%d\033[0m \033[31m↓%d\033[0m",
                 ahead, behind);
        strncat(buf, extra, sizeof(buf) - strlen(buf) - 1);
    }

    free(branch);
    return strdup(buf);
}

static Plugin git_plugin = {
    .name            = "git",
    .version         = "1.0",
    .on_prompt_left  = git_prompt_left,
    .on_prompt_right = NULL,
    .on_pre_exec     = NULL,
    .on_post_exec    = NULL,
};

Plugin *plugin_register(void) {
    return &git_plugin;
}