//
// Created by mete on 27.04.2026.
//



#ifndef INPUT_H
#define INPUT_H

#define MAXIMUM_INPUT 4096


char *read_line(const char *prompt);

/* history.c */
void  history_init(const char *db_path);
void  history_add(const char *line);
char *history_get(int offset);
char *history_search_prefix(const char *prefix);  /* yeni fonksiyon */
char *history_search(const char *query, int skip);
int   history_count(void);
void  history_close(void);

#endif

