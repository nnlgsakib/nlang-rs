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
