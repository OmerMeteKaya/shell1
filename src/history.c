//
// Created by mete on 23.04.2026.
//

#include <sqlite3.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include "../include/input.h"

static sqlite3 *db = NULL;

void history_init(const char *db_path) {
    if (sqlite3_open(db_path, &db) != SQLITE_OK) {
        return;
    }
    
    const char *sql = "CREATE TABLE IF NOT EXISTS history ("
                      "id   INTEGER PRIMARY KEY AUTOINCREMENT,"
                      "cmd  TEXT NOT NULL,"
                      "ts   INTEGER DEFAULT (strftime('%s','now'))"
                      ");";
    
    sqlite3_exec(db, sql, NULL, NULL, NULL);
}

void history_add(const char *line) {
    if (!line || !db) return;
    
    // Boş veya sadece boşluk karakterlerinden oluşan satırları ekleme
    const char *p = line;
    while (*p && isspace((unsigned char)*p)) p++;
    if (*p == '\0') return;
    
    const char *sql = "INSERT INTO history (cmd) VALUES (?);";
    sqlite3_stmt *stmt;
    
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, NULL) != SQLITE_OK) {
        return;
    }
    
    sqlite3_bind_text(stmt, 1, line, -1, SQLITE_STATIC);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

char *history_get(int offset) {
    if (!db || offset < 1) return NULL;
    
    const char *sql = "SELECT cmd FROM history ORDER BY id DESC LIMIT 1 OFFSET ?;";
    sqlite3_stmt *stmt;
    
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, NULL) != SQLITE_OK) {
        return NULL;
    }
    
    sqlite3_bind_int(stmt, 1, offset - 1);
    
    if (sqlite3_step(stmt) == SQLITE_ROW) {
        const char *cmd = (const char *)sqlite3_column_text(stmt, 0);
        char *result = strdup(cmd);
        sqlite3_finalize(stmt);
        return result;
    }
    
    sqlite3_finalize(stmt);
    return NULL;
}

char *history_search_prefix(const char *prefix) {
    if (!db || !prefix || !*prefix) return NULL;
    
    const char *sql = "SELECT cmd FROM history WHERE cmd LIKE ? || '%' ORDER BY id DESC LIMIT 1;";
    sqlite3_stmt *stmt;
    
    if (sqlite3_prepare_v2(db, sql, -1, &stmt, NULL) != SQLITE_OK) {
        return NULL;
    }
    
    sqlite3_bind_text(stmt, 1, prefix, -1, SQLITE_STATIC);
    
    if (sqlite3_step(stmt) == SQLITE_ROW) {
        const char *cmd = (const char *)sqlite3_column_text(stmt, 0);
        char *result = strdup(cmd);
        sqlite3_finalize(stmt);
        return result;
    }
    
    sqlite3_finalize(stmt);
    return NULL;
}

void history_close(void) {
    if (db) {
        sqlite3_close(db);
        db = NULL;
    }
}
