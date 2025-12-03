#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <stdarg.h>
#include <math.h>
#include <stdint.h>
#include <time.h>
#include <locale.h>
#include <ctype.h>
#include <stddef.h>
#ifdef _WIN32
#include <windows.h>
#include <io.h>
#include <fcntl.h>
#endif

// Set up UTF-8 support for proper Unicode display
#ifdef _WIN32
#ifndef ENABLE_VIRTUAL_TERMINAL_PROCESSING
#define ENABLE_VIRTUAL_TERMINAL_PROCESSING 0x0004
#endif
static void setup_utf8_console() {
 // Set console to UTF-8 mode
 SetConsoleOutputCP(CP_UTF8);
 SetConsoleCP(CP_UTF8);
 // Enable virtual terminal processing for ANSI escape codes (optional)
 HANDLE hStdOut = GetStdHandle(STD_OUTPUT_HANDLE);
 DWORD dwMode = 0;
 GetConsoleMode(hStdOut, &dwMode);
 SetConsoleMode(hStdOut, dwMode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
}
#else
static void setup_utf8_console() {
 setlocale(LC_ALL, "en_US.UTF-8");
}
#endif

// Built-in string conversion functions
char* int_to_str(int64_t value) {
 static char buffer[20];
 snprintf(buffer, sizeof(buffer), "%lld", value);
 return buffer;
}

char* float_to_str(double value) {
 static char buffer[50];
 snprintf(buffer, sizeof(buffer), "%g", value);
 return buffer;
}

char* read_line(const char* prompt) {
 if (prompt) {
  printf("%s", prompt);
  fflush(stdout);
 }

 size_t buffer_size = 128;
 char* buffer = malloc(buffer_size);
 if (!buffer) return NULL;

 int c;
 size_t position = 0;
 while (1) {
  c = getchar();
  if (c == EOF || c == '\n') {
   buffer[position] = '\0';
   return buffer;
  } else {
   buffer[position] = c;
  }
  position++;

  if (position >= buffer_size) {
   buffer_size += 128;
   char* new_buffer = realloc(buffer, buffer_size);
   if (!new_buffer) { free(buffer); return NULL; }
   buffer = new_buffer;
  }
 }
}

char* str_concat(const char* s1, const char* s2) {
 size_t len1 = strlen(s1);
 size_t len2 = strlen(s2);
 char* result = malloc(len1 + len2 + 1);
 if (result) {
  memcpy(result, s1, len1);
  memcpy(result + len1, s2, len2);
  result[len1 + len2] = '\0';
 }
 return result;
}

// Library: env_man
// Standard headers are already included by the Nlang compiler

#ifdef _WIN32
#define ENV_MAN_EXPORT __declspec(dllexport)
#else
#include <unistd.h>
#define ENV_MAN_EXPORT
extern char **environ;
#endif

// Helper to create a new string copy
char* env_man_strdup(const char* s) {
    if (!s) return NULL;
    size_t len = strlen(s) + 1;
    char* new_s = (char*)malloc(len);
    if (new_s) memcpy(new_s, s, len);
    return new_s;
}

// Get environment variable
char* env_man_get(const char* key) {
    if (!key) return env_man_strdup("");
#ifdef _WIN32
    DWORD len = GetEnvironmentVariable(key, NULL, 0);
    if (len > 0) {
        char* buf = (char*)malloc(len);
        if (buf) {
            GetEnvironmentVariable(key, buf, len);
            return buf;
        }
    }
    return env_man_strdup("");
#else
    char* val = getenv(key);
    if (val) return env_man_strdup(val);
    return env_man_strdup("");
#endif
}

// Set environment variable
void env_man_set(const char* key, const char* value) {
    if (!key || !value) return;
#ifdef _WIN32
    SetEnvironmentVariable(key, value);
#else
    setenv(key, value, 1);
#endif
}

// Unset environment variable
void env_man_unset(const char* key) {
    if (!key) return;
#ifdef _WIN32
    SetEnvironmentVariable(key, NULL);
#else
    unsetenv(key);
#endif
}

// List all environment variables
char* env_man_list() {
    // Estimate size or use dynamic resizing
    size_t capacity = 1024;
    size_t len = 0;
    char* result = (char*)malloc(capacity);
    if (!result) return NULL;
    result[0] = '\0';

#ifdef _WIN32
    char* env_block = GetEnvironmentStrings();
    if (!env_block) {
        free(result);
        return env_man_strdup("");
    }

    char* var = env_block;
    while (*var) {
        size_t var_len = strlen(var);
        if (len + var_len + 2 > capacity) {
            capacity *= 2;
            if (capacity < len + var_len + 2) capacity = len + var_len + 2 + 1024;
            char* new_res = (char*)realloc(result, capacity);
            if (!new_res) {
                FreeEnvironmentStrings(env_block);
                free(result);
                return NULL;
            }
            result = new_res;
        }
        strcat(result, var);
        strcat(result, "\n");
        len += var_len + 1;
        var += var_len + 1;
    }
    FreeEnvironmentStrings(env_block);
#else
    for (char **env = environ; *env != 0; env++) {
        char* var = *env;
        size_t var_len = strlen(var);
        if (len + var_len + 2 > capacity) {
            capacity *= 2;
            if (capacity < len + var_len + 2) capacity = len + var_len + 2 + 1024;
            char* new_res = (char*)realloc(result, capacity);
            if (!new_res) {
                free(result);
                return NULL;
            }
            result = new_res;
        }
        strcat(result, var);
        strcat(result, "\n");
        len += var_len + 1;
    }
#endif
    return result;
}

// Get OS name
char* env_man_os() {
#ifdef _WIN32
    return env_man_strdup("windows");
#elif __APPLE__
    return env_man_strdup("macos");
#elif __linux__
    return env_man_strdup("linux");
#else
    return env_man_strdup("unknown");
#endif
}

// Load environment variables from file
void env_man_load_env(const char* filename) {
    FILE* f = fopen(filename, "r");
    if (!f) return;

    char line[4096];
    while (fgets(line, sizeof(line), f)) {
        size_t len = strlen(line);
        while (len > 0 && (line[len-1] == '\n' || line[len-1] == '\r')) {
            line[len-1] = '\0';
            len--;
        }

        if (len == 0 || line[0] == '#') continue;

        char* eq = strchr(line, '=');
        if (eq) {
            *eq = '\0';
            char* key = line;
            char* val = eq + 1;
            env_man_set(key, val);
        }
    }
    fclose(f);
}


static const char str_const_1[] = "";
static const char str_const_2[] = "Hero";
static const char str_const_3[] = "Player: ";
static const char str_const_4[] = "Health: ";
static const char str_const_5[] = "Emote Actions:";
static const char str_const_0[] = "🎮 Welcome to NLang Game!";


// Math helpers for std lib mapping
static char* str_upper(const char* s){
 size_t n = strlen(s);
 char* out = (char*)malloc(n+1);
 if(!out) return NULL;
 for(size_t i=0;i<n;i++){ out[i] = (char)toupper((unsigned char)s[i]); }
 out[n] = 0;
 return out;
}
static char* str_lower(const char* s){
 size_t n = strlen(s);
 char* out = (char*)malloc(n+1);
 if(!out) return NULL;
 for(size_t i=0;i<n;i++){ out[i] = (char)tolower((unsigned char)s[i]); }
 out[n] = 0;
 return out;
}
static char* str_trim(const char* s){
 size_t n = strlen(s);
 size_t start = 0; while(start < n && isspace((unsigned char)s[start])) start++;
 size_t end = n; while(end>start && isspace((unsigned char)s[end-1])) end--;
 size_t m = end - start;
 char* out = (char*)malloc(m+1);
 if(!out) return NULL;
 memcpy(out, s+start, m);
 out[m] = 0;
 return out;
}
static int str_contains(const char* hay, const char* needle){
 return strstr(hay, needle) != NULL;
}
int main(void);

int main(void) {
 setup_utf8_console();
 printf("%s\n", str_const_0);
 printf("%s\n", str_const_1);
 const char* player_name = game_character_create_character(str_const_2);
 int64_t health = game_character_get_health();
 printf("%s", str_const_3);
 printf("%s\n", player_name);
 printf("%s", str_const_4);
 printf("%lld\n", health);
 printf("%s\n", str_const_1);
 printf("%s\n", str_const_5);
 game_emotes_wave();
 game_emotes_dance();
 game_emotes_cheer();
 return 0;
}

