//
// Created by mete on 23.04.2026.
//

#include <time.h>
extern int last_exit_status;

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>

#include "../include/shell.h"
#include "../include/input.h"
#include "../include/alias.h"
#include "../include/rc.h"
#include "../include/plugin.h"

void signals_init(void);
void jobs_init(void);



int main() {

    
    // Initialize signals and jobs
    signals_init();
    jobs_init();
    char history_path[512];
    const char *home = getenv("HOME");
    if (home) {
        snprintf(history_path, sizeof(history_path), "%s/.mysh_history", home);
    } else {
        snprintf(history_path, sizeof(history_path), ".mysh_history");
    }
    history_init(history_path);

    // Alias
    alias_init();

    // Plugin
    char plugin_dir[512];
    if (home)
        snprintf(plugin_dir, sizeof(plugin_dir), "%s/.mysh/plugins", home);
    else
        snprintf(plugin_dir, sizeof(plugin_dir), ".mysh/plugins");
    plugins_init(plugin_dir);
    char rc_path[512];
    if (home) {
        snprintf(rc_path, sizeof(rc_path), "%s/.myshrc", home);
    } else {
        snprintf(rc_path, sizeof(rc_path), ".myshrc");
    }
    rc_load(rc_path);

    // Take shell process group ownership
    pid_t shell_pgid = getpid();
    setpgid(shell_pgid, shell_pgid);
    tcsetpgrp(STDIN_FILENO, shell_pgid);
    
    
    while (1) {
        char prompt[512];
        char cwd[256];
        const char *home = getenv("HOME");

        if (getcwd(cwd, sizeof(cwd))) {
            char display[256];
            if (home && strcmp(cwd, home) == 0) {
                strncpy(display, "~", sizeof(display));
            } else {
                char *last = strrchr(cwd, '/');
                if (last && *(last+1))
                    strncpy(display, last+1, sizeof(display));
                else
                    strncpy(display, cwd, sizeof(display));
            }

            /* plugin hook — git branch vb. */
            char *left = hook_prompt_left();
            if (left) {
                /* git varsa: ➜  HH:MM user dir (branch) */
                /* saat */
                time_t t = time(NULL);
                struct tm *tm = localtime(&t);
                char timebuf[8];
                strftime(timebuf, sizeof(timebuf), "%H:%M", tm);

                snprintf(prompt, sizeof(prompt),
                    "\033[1;32m➜\033[0m  "        /* yeşil ok */
                    "\033[0;37m%s\033[0m "         /* gri saat */
                    "\033[1;36m%s\033[0m "         /* cyan kullanıcı */
                    "\033[1;34m%s\033[0m "         /* mavi dizin */
                    "%s"                           /* git (zaten renkli) */
                    "\033[0m> ",                   /* reset + > */
                    timebuf,
                    getenv("USER") ? getenv("USER") : "",
                    display,
                    left);
                free(left);
            } else {
                time_t t = time(NULL);
                struct tm *tm = localtime(&t);
                char timebuf[8];
                strftime(timebuf, sizeof(timebuf), "%H:%M", tm);

                snprintf(prompt, sizeof(prompt),
                    "\033[1;32m➜\033[0m  "
                    "\033[0;37m%s\033[0m "
                    "\033[1;36m%s\033[0m "
                    "\033[1;34m%s\033[0m> ",
                    timebuf,
                    getenv("USER") ? getenv("USER") : "",
                    display);
            }
        } else {
            snprintf(prompt, sizeof(prompt), "➜ mysh> ");
        }
        char *input = read_line(prompt);
        if (!input) {
            printf("\n");
            break;
        }
        /* strip trailing newline — artık gerekmiyor ama zarar vermez */
        input[strcspn(input, "\n")] = '\0';
        if (strlen(input) == 0) { free(input); continue; }
        
        // Strip trailing newline
        input[strcspn(input, "\n")] = '\0';
        
        // Skip empty lines
        if (strlen(input) == 0) {
            continue;
        }
        
        // Lexical analysis
        int ntokens;
        Token *tokens = lex(input, &ntokens);
        if (!tokens) {
            fprintf(stderr, "Error during lexical analysis\n");
            continue;
        }

        tokens = glob_expand_tokens(tokens, &ntokens, last_exit_status);
       /* for (int i = 0; i < ntokens; i++)
            fprintf(stderr, "TOK[%d]: type=%d val=%s\n", i, tokens[i].type, tokens[i].value ? tokens[i].value : "null");
        if (!tokens) continue;*/

        if (ntokens > 0 && tokens[0].type == TOK_WORD && tokens[0].value) {
            char *expanded = alias_expand(tokens[0].value);
            if (expanded) {

                char combined[4096];
                snprintf(combined, sizeof(combined), "%s", expanded);
                for (int i = 1; i < ntokens - 1; i++) { /* -1 for EOF */
                    if (tokens[i].value) {
                        strncat(combined, " ", sizeof(combined)-strlen(combined)-1);
                        strncat(combined, tokens[i].value, sizeof(combined)-strlen(combined)-1);
                    }
                }
                tokens_free(tokens, ntokens);
                tokens = lex(combined, &ntokens);
                if (!tokens) continue;
                tokens = glob_expand_tokens(tokens, &ntokens, last_exit_status);
                if (!tokens) continue;
            }
        }

        // Execution
        CmdList *list = parse_list(tokens, ntokens);
        if (list) {
            execute_list(list);
            cmdlist_free(list);
        }
        // Cleanup
        tokens_free(tokens, ntokens);
        free(input);
    }
    history_close();
    alias_free();
    plugins_unload();
    return 0;
}
