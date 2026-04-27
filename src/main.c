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
    history_init("~/.mysh_history");
    
    // Take shell process group ownership
    pid_t shell_pgid = getpid();
    setpgid(shell_pgid, shell_pgid);
    tcsetpgrp(STDIN_FILENO, shell_pgid);
    
    
    while (1) {
        printf("mysh> ");
        fflush(stdout);

        char *input = read_line("mysh> ");
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
