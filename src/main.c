//
// Created by mete on 23.04.2026.
//

extern int last_exit_status;

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <unistd.h>

#include "../include/shell.h"
#include "../include/input.h"

void signals_init(void);
void jobs_init(void);



int main() {

    char input[MAX_INPUT];
    
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
    // Take shell process group ownership
    pid_t shell_pgid = getpid();
    setpgid(shell_pgid, shell_pgid);
    tcsetpgrp(STDIN_FILENO, shell_pgid);
    
    
    while (1) {
        printf("mysh> ");
        fflush(stdout);

        char prompt[512];
        char cwd[256];
        if (getcwd(cwd, sizeof(cwd))) {
            const char *home = getenv("HOME");
            char display[256];
            if (home && strncmp(cwd, home, strlen(home)) == 0) {
                snprintf(display, sizeof(display), "~%s", cwd + strlen(home));
            } else {
                strncpy(display, cwd, sizeof(display));
            }
            snprintf(prompt, sizeof(prompt), "%s> ", display);
        } else {
            snprintf(prompt, sizeof(prompt), "mysh> ");
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
        if (!tokens) continue;

        // Parsing
        Pipeline *pipeline = parse(tokens, ntokens);
        if (!pipeline) {
            tokens_free(tokens, ntokens);
            continue;
        }
        
        // Execution
        if (pipeline->ncommands > 0 && 
            pipeline->commands[0].argc > 0 && 
            is_builtin(pipeline->commands[0].argv[0])) {
            // Run built-in command
            run_builtin(&pipeline->commands[0]);
        } else {
            // Execute external commands
            execute(pipeline);
        }
        
        // Cleanup
        pipeline_free(pipeline);
        tokens_free(tokens, ntokens);
        free(input);
    }
    history_close();
    
    return 0;
}
