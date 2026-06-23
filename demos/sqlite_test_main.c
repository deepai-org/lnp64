#include "sqlite3.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int count_callback(void *data, int argc, char **argv, char **col_names) {
  int *count = (int *)data;
  (*count)++;
  if (argc >= 2) {
    printf("  Row %d: id=%s name=%s\n", *count,
           argv[0] ? argv[0] : "NULL",
           argv[1] ? argv[1] : "NULL");
  }
  return 0;
}

int main(void) {
  sqlite3 *db = 0;
  char *errmsg = 0;
  int rc;

  printf("SQLite %s in-memory database test\n", sqlite3_libversion());

  /* Explicitly initialize SQLite */
  rc = sqlite3_initialize();
  if (rc != SQLITE_OK) {
    printf("FAIL: sqlite3_initialize returned %d\n", rc);
    return 1;
  }
  printf("OK: sqlite3_initialize\n");

  rc = sqlite3_open(":memory:", &db);
  if (rc != SQLITE_OK) {
    printf("FAIL: sqlite3_open returned %d: %s\n", rc, sqlite3_errmsg(db));
    return 1;
  }
  printf("OK: opened :memory: database\n");

  printf("DB ptr: %p\n", (void *)db);
  printf("SQLite library version: %s\n", sqlite3_libversion());

  /* Test basic SQL first */
  printf("Attempting CREATE TABLE test (x INT)...\n");
  rc = sqlite3_exec(db, "CREATE TABLE test (x INT);", 0, 0, &errmsg);
  if (rc != SQLITE_OK) {
    printf("FAIL: CREATE TABLE test: %s\n", errmsg ? errmsg : "(null)");
    sqlite3_close(db);
    return 1;
  }
  printf("OK: CREATE TABLE test\n");

  printf("Attempting CREATE TABLE users...\n");
  rc = sqlite3_exec(db,
    "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
    0, 0, &errmsg);
  if (rc != SQLITE_OK) {
    printf("FAIL: CREATE TABLE: %s\n", errmsg ? errmsg : "?");
    sqlite3_close(db);
    return 1;
  }
  printf("OK: CREATE TABLE users\n");

  rc = sqlite3_exec(db,
    "INSERT INTO users VALUES (1, 'alice');"
    "INSERT INTO users VALUES (2, 'bob');"
    "INSERT INTO users VALUES (3, 'charlie');",
    0, 0, &errmsg);
  if (rc != SQLITE_OK) {
    printf("FAIL: INSERT: %s\n", errmsg ? errmsg : "?");
    sqlite3_close(db);
    return 1;
  }
  printf("OK: inserted 3 rows\n");

  int count = 0;
  rc = sqlite3_exec(db, "SELECT id, name FROM users ORDER BY id;",
    count_callback, &count, &errmsg);
  if (rc != SQLITE_OK) {
    printf("FAIL: SELECT: %s\n", errmsg ? errmsg : "?");
    sqlite3_close(db);
    return 1;
  }
  printf("OK: SELECT returned %d rows\n", count);

  if (count != 3) {
    printf("FAIL: expected 3 rows, got %d\n", count);
    sqlite3_close(db);
    return 1;
  }

  sqlite3_close(db);
  printf("SQLite in-memory database test passed\n");
  printf("exit=0\n");
  return 0;
}
