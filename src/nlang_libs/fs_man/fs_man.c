// fs_man C implementation - Cross-platform file system operations
// Standard headers are included by the Nlang compiler

#ifdef _WIN32
#include <windows.h>
#include <sys/stat.h>
#include <io.h>
#include <direct.h>
#include <process.h>
#define FS_MAN_EXPORT __declspec(dllexport)
#define PATH_SEPARATOR '\\'
#define PATH_MAX 260
#define fs_mkdir(path, mode) _mkdir(path)
typedef struct _stat fs_stat_t;
#define fs_stat _stat
#ifndef S_ISDIR
#define S_ISDIR(mode) (((mode) & S_IFMT) == S_IFDIR)
#endif
#ifndef S_ISREG
#define S_ISREG(mode) (((mode) & S_IFMT) == S_IFREG)
#endif
#ifndef S_ISLNK
#define S_ISLNK(mode) 0
#endif
#else
#include <unistd.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include <limits.h>
#define FS_MAN_EXPORT
#define PATH_SEPARATOR '/'
#ifndef PATH_MAX
#define PATH_MAX 4096
#endif
#define fs_mkdir(path, mode) mkdir(path, mode)
typedef struct stat fs_stat_t;
#define fs_stat stat
#endif

typedef struct {
    char** items;
    size_t count;
} StringArray;

char* fs_man_strdup(const char* s) {
    if (!s) return NULL;
    size_t len = strlen(s) + 1;
    char* new_s = (char*)malloc(len);
    if (new_s) memcpy(new_s, s, len);
    return new_s;
}

int fs_man_exists(const char* path) {
    if (!path) return 0;
#ifdef _WIN32
    DWORD attrs = GetFileAttributes(path);
    return (attrs != INVALID_FILE_ATTRIBUTES);
#else
    return (access(path, F_OK) == 0);
#endif
}

int fs_man_is_file(const char* path) {
    if (!path) return 0;
    fs_stat_t st;
    if (fs_stat(path, &st) != 0) return 0;
    return S_ISREG(st.st_mode);
}

int fs_man_is_dir(const char* path) {
    if (!path) return 0;
    fs_stat_t st;
    if (fs_stat(path, &st) != 0) return 0;
    return S_ISDIR(st.st_mode);
}

char* fs_man_join(const char* path1, const char* path2) {
    if (!path1 || !path2) return fs_man_strdup("");
    
    size_t len1 = strlen(path1);
    size_t len2 = strlen(path2);
    
    int need_sep = 0;
    if (len1 > 0 && len2 > 0) {
        if (path1[len1-1] != PATH_SEPARATOR && path2[0] != PATH_SEPARATOR) {
            need_sep = 1;
        }
    }
    
    size_t total_len = len1 + len2 + need_sep + 1;
    char* result = (char*)malloc(total_len);
    if (!result) return NULL;
    
    strcpy(result, path1);
    if (need_sep) {
        result[len1] = PATH_SEPARATOR;
        result[len1 + 1] = '\0';
        strcat(result, path2);
    } else {
        strcat(result, path2);
    }
    
    return result;
}

char* fs_man_basename(const char* path) {
    if (!path) return fs_man_strdup("");
    
    const char* last_sep = strrchr(path, PATH_SEPARATOR);
#ifdef _WIN32
    const char* last_fwd = strrchr(path, '/');
    if (last_fwd > last_sep) last_sep = last_fwd;
#endif
    
    if (last_sep) return fs_man_strdup(last_sep + 1);
    return fs_man_strdup(path);
}

char* fs_man_dirname(const char* path) {
    if (!path) return fs_man_strdup("");
    
    char* path_copy = fs_man_strdup(path);
    if (!path_copy) return NULL;
    
    char* last_sep = strrchr(path_copy, PATH_SEPARATOR);
#ifdef _WIN32
    char* last_fwd = strrchr(path_copy, '/');
    if (last_fwd > last_sep) last_sep = last_fwd;
#endif
    
    if (last_sep) {
        *last_sep = '\0';
        return path_copy;
    }
    
    free(path_copy);
    return fs_man_strdup(".");
}

char* fs_man_extname(const char* path) {
    if (!path) return fs_man_strdup("");
    
    const char* last_sep = strrchr(path, PATH_SEPARATOR);
#ifdef _WIN32
    const char* last_fwd = strrchr(path, '/');
    if (last_fwd > last_sep) last_sep = last_fwd;
#endif
    
    const char* basename = last_sep ? last_sep + 1 : path;
    const char* dot = strrchr(basename, '.');
    
    if (dot && dot != basename) {
        return fs_man_strdup(dot);
    }
    
    return fs_man_strdup("");
}

int fs_man_create_path_recursive(const char* path) {
    if (!path || strlen(path) == 0) return 0;
    
    if (fs_man_is_dir(path)) return 1;
    
    char* path_copy = fs_man_strdup(path);
    if (!path_copy) return 0;
    
    char* sep = strrchr(path_copy, PATH_SEPARATOR);
#ifdef _WIN32
    char* fwd = strrchr(path_copy, '/');
    if (fwd > sep) sep = fwd;
#endif
    
    if (sep) {
        *sep = '\0';
        if (!fs_man_create_path_recursive(path_copy)) {
            free(path_copy);
            return 0;
        }
        *sep = PATH_SEPARATOR;
    }
    
    int result = (fs_mkdir(path, 0755) == 0 || fs_man_is_dir(path));
    free(path_copy);
    return result;
}

void fs_man_create_path(const char* path) {
    if (!path) return;
    fs_man_create_path_recursive(path);
}

int fs_man_remove_dir_recursive(const char* path) {
#ifdef _WIN32
    WIN32_FIND_DATA find_data;
    char search_path[MAX_PATH];
    snprintf(search_path, MAX_PATH, "%s\\*", path);
    
    HANDLE h_find = FindFirstFile(search_path, &find_data);
    if (h_find == INVALID_HANDLE_VALUE) return 0;
    
    do {
        if (strcmp(find_data.cFileName, ".") == 0 || strcmp(find_data.cFileName, "..") == 0) {
            continue;
        }
        
        char full_path[MAX_PATH];
        snprintf(full_path, MAX_PATH, "%s\\%s", path, find_data.cFileName);
        
        if (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            if (!fs_man_remove_dir_recursive(full_path)) {
                FindClose(h_find);
                return 0;
            }
        } else {
            if (!DeleteFile(full_path)) {
                FindClose(h_find);
                return 0;
            }
        }
    } while (FindNextFile(h_find, &find_data));
    
    FindClose(h_find);
    return RemoveDirectory(path);
#else
    DIR* dir = opendir(path);
    if (!dir) return 0;
    
    struct dirent* entry;
    int result = 1;
    
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        
        char full_path[PATH_MAX];
        snprintf(full_path, PATH_MAX, "%s/%s", path, entry->d_name);
        
        fs_stat_t st;
        if (fs_stat(full_path, &st) != 0) {
            result = 0;
            break;
        }
        
        if (S_ISDIR(st.st_mode)) {
            if (!fs_man_remove_dir_recursive(full_path)) {
                result = 0;
                break;
            }
        } else {
            if (unlink(full_path) != 0) {
                result = 0;
                break;
            }
        }
    }
    
    closedir(dir);
    if (result) {
        result = (rmdir(path) == 0);
    }
    return result;
#endif
}

void fs_man_remove_path(const char* path) {
    if (!path || !fs_man_exists(path)) return;
    
    if (fs_man_is_dir(path)) {
        fs_man_remove_dir_recursive(path);
    } else {
#ifdef _WIN32
        DeleteFile(path);
#else
        unlink(path);
#endif
    }
}

int64_t* fs_man_file_stat(const char* path) {
    if (!path) return NULL;
    
    fs_stat_t st;
    if (fs_stat(path, &st) != 0) return NULL;
    
    int64_t* result = (int64_t*)malloc(5 * sizeof(int64_t));
    if (!result) return NULL;
    
    result[0] = (int64_t)st.st_size;
    result[1] = S_ISREG(st.st_mode) ? 1 : 0;
    result[2] = S_ISDIR(st.st_mode) ? 1 : 0;
    result[3] = (int64_t)st.st_mtime;
    
#ifdef _WIN32
    result[4] = (st.st_mode & _S_IWRITE) ? 0666 : 0444;
#else
    result[4] = (int64_t)st.st_mode;
#endif
    
    return result;
}

int64_t fs_man_file_size(const char* path) {
    if (!path) return -1;
    
    fs_stat_t st;
    if (fs_stat(path, &st) != 0) return -1;
    return (int64_t)st.st_size;
}

double fs_man_last_modified(const char* path) {
    if (!path) return -1.0;
    
    fs_stat_t st;
    if (fs_stat(path, &st) != 0) return -1.0;
    return (double)st.st_mtime;
}

int64_t fs_man_permissions(const char* path) {
    if (!path) return -1;
    
    fs_stat_t st;
    if (fs_stat(path, &st) != 0) return -1;
    
#ifdef _WIN32
    return (st.st_mode & _S_IWRITE) ? 0666 : 0444;
#else
    return (int64_t)st.st_mode;
#endif
}

void fs_man_set_permissions(const char* path, int64_t mode) {
    if (!path) return;
    
#ifdef _WIN32
    int readonly = ((mode & 0200) == 0);
    DWORD attrs = GetFileAttributes(path);
    if (attrs != INVALID_FILE_ATTRIBUTES) {
        if (readonly) {
            attrs |= FILE_ATTRIBUTE_READONLY;
        } else {
            attrs &= ~FILE_ATTRIBUTE_READONLY;
        }
        SetFileAttributes(path, attrs);
    }
#else
    chmod(path, (mode_t)mode);
#endif
}

char* fs_man_read_file(const char* path) {
    if (!path) return NULL;
    FILE* f = fopen(path, "rb");
    if (!f) return NULL;
    
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    
    char* buffer = (char*)malloc(size + 1);
    if (!buffer) {
        fclose(f);
        return NULL;
    }
    
    size_t read = fread(buffer, 1, size, f);
    buffer[read] = '\0';
    fclose(f);
    
    return buffer;
}

void fs_man_write_file(const char* path, const char* content) {
    if (!path || !content) return;
    FILE* f = fopen(path, "wb");
    if (!f) return;
    fwrite(content, 1, strlen(content), f);
    fclose(f);
}

void fs_man_append_file(const char* path, const char* content) {
    if (!path || !content) return;
    FILE* f = fopen(path, "ab");
    if (!f) return;
    fwrite(content, 1, strlen(content), f);
    fclose(f);
}

void fs_man_copy_file(const char* src, const char* dst) {
    if (!src || !dst) return;
    
    FILE* fsrc = fopen(src, "rb");
    if (!fsrc) return;
    
    FILE* fdst = fopen(dst, "wb");
    if (!fdst) {
        fclose(fsrc);
        return;
    }
    
    char buffer[4096];
    size_t n;
    while ((n = fread(buffer, 1, sizeof(buffer), fsrc)) > 0) {
        fwrite(buffer, 1, n, fdst);
    }
    
    fclose(fsrc);
    fclose(fdst);
}

void fs_man_move_file(const char* src, const char* dst) {
    if (!src || !dst) return;
    rename(src, dst);
}

void fs_man_rename_file(const char* src, const char* dst) {
    fs_man_move_file(src, dst);
}

void fs_man_create_file(const char* path) {
    if (!path) return;
    FILE* f = fopen(path, "wb");
    if (f) fclose(f);
}

void fs_man_remove_file(const char* path) {
    if (!path) return;
#ifdef _WIN32
    DeleteFile(path);
#else
    unlink(path);
#endif
}

void fs_man_remove_dir(const char* path) {
    if (!path) return;
#ifdef _WIN32
    RemoveDirectory(path);
#else
    rmdir(path);
#endif
}

void fs_man_remove_dir_all(const char* path) {
    if (!path) return;
    fs_man_remove_dir_recursive(path);
}

void fs_man_create_dir(const char* path) {
    if (!path) return;
    fs_mkdir(path, 0755);
}

void fs_man_create_dir_all(const char* path) {
    fs_man_create_path(path);
}

char* fs_man_current_dir() {
    char* buffer = (char*)malloc(PATH_MAX);
    if (!buffer) return NULL;
    
#ifdef _WIN32
    if (GetCurrentDirectory(PATH_MAX, buffer) == 0) {
        free(buffer);
        return NULL;
    }
#else
    if (getcwd(buffer, PATH_MAX) == NULL) {
        free(buffer);
        return NULL;
    }
#endif
    
    return buffer;
}

char* fs_man_temp_dir() {
#ifdef _WIN32
    char* buffer = (char*)malloc(PATH_MAX);
    if (!buffer) return NULL;
    if (GetTempPath(PATH_MAX, buffer) == 0) {
        free(buffer);
        return NULL;
    }
    return buffer;
#else
    const char* tmp = getenv("TMPDIR");
    if (!tmp) tmp = getenv("TMP");
    if (!tmp) tmp = getenv("TEMP");
    if (!tmp) tmp = "/tmp";
    return fs_man_strdup(tmp);
#endif
}

char* fs_man_home_dir() {
#ifdef _WIN32
    const char* home = getenv("USERPROFILE");
    if (!home) {
        const char* drive = getenv("HOMEDRIVE");
        const char* path = getenv("HOMEPATH");
        if (drive && path) {
            char* result = (char*)malloc(strlen(drive) + strlen(path) + 1);
            if (result) {
                strcpy(result, drive);
                strcat(result, path);
                return result;
            }
        }
    }
    return home ? fs_man_strdup(home) : fs_man_strdup("");
#else
    const char* home = getenv("HOME");
    return home ? fs_man_strdup(home) : fs_man_strdup("");
#endif
}

char* fs_man_get_path_separator() {
#ifdef _WIN32
    return fs_man_strdup("\\");
#else
    return fs_man_strdup("/");
#endif
}

int fs_man_is_symlink(const char* path) {
    if (!path) return 0;
#ifdef _WIN32
    DWORD attrs = GetFileAttributes(path);
    if (attrs == INVALID_FILE_ATTRIBUTES) return 0;
    return (attrs & FILE_ATTRIBUTE_REPARSE_POINT) != 0;
#else
    fs_stat_t st;
    if (lstat(path, &st) != 0) return 0;
    return S_ISLNK(st.st_mode);
#endif
}

char* fs_man_read_link(const char* path) {
    if (!path) return fs_man_strdup("");
#ifdef _WIN32
    return fs_man_strdup("");
#else
    char buffer[PATH_MAX];
    ssize_t len = readlink(path, buffer, sizeof(buffer) - 1);
    if (len < 0) return fs_man_strdup("");
    buffer[len] = '\0';
    return fs_man_strdup(buffer);
#endif
}

void fs_man_create_symlink(const char* target, const char* link) {
    if (!target || !link) return;
#ifdef _WIN32
    // CreateSymbolicLink requires Windows Vista+ and special privileges
    // For compatibility, just create a file that contains the target path
    FILE* f = fopen(link, "w");
    if (f) {
        fprintf(f, "%s", target);
        fclose(f);
    }
#else
    symlink(target, link);
#endif
}

void fs_man_create_hard_link(const char* target, const char* link) {
    if (!target || !link) return;
#ifdef _WIN32
    CreateHardLink(link, target, NULL);
#else
    link(target, link);
#endif
}

char* fs_man_canonicalize(const char* path) {
    if (!path) return fs_man_strdup("");
#ifdef _WIN32
    char buffer[PATH_MAX];
    if (_fullpath(buffer, path, PATH_MAX) == NULL) {
        return fs_man_strdup(path);
    }
    return fs_man_strdup(buffer);
#else
    char* resolved = realpath(path, NULL);
    if (!resolved) return fs_man_strdup(path);
    char* result = fs_man_strdup(resolved);
    free(resolved);
    return result;
#endif
}

char* fs_man_absolute_path(const char* path) {
    if (!path) return fs_man_strdup("");
    
    fs_stat_t st;
    if (fs_stat(path, &st) == 0) {
        return fs_man_canonicalize(path);
    }
    
    char* cwd = fs_man_current_dir();
    if (!cwd) return fs_man_strdup(path);
    
    size_t len = strlen(cwd) + strlen(path) + 2;
    char* result = (char*)malloc(len);
    if (!result) {
        free(cwd);
        return fs_man_strdup(path);
    }
    
#ifdef _WIN32
    snprintf(result, len, "%s\\%s", cwd, path);
#else
    snprintf(result, len, "%s/%s", cwd, path);
#endif
    
    free(cwd);
    return result;
}

int fs_man_is_absolute(const char* path) {
    if (!path || strlen(path) == 0) return 0;
#ifdef _WIN32
    return (strlen(path) >= 3 && path[1] == ':' && (path[2] == '\\' || path[2] == '/'));
#else
    return (path[0] == '/');
#endif
}

int fs_man_is_relative(const char* path) {
    return !fs_man_is_absolute(path);
}

void fs_man_set_current_dir(const char* path) {
    if (!path) return;
#ifdef _WIN32
    SetCurrentDirectory(path);
#else
    chdir(path);
#endif
}

char* fs_man_create_temp_file(const char* prefix) {
    if (!prefix) prefix = "tmp";
    
    char* tmp_dir = fs_man_temp_dir();
    if (!tmp_dir) return NULL;
    
    unsigned long timestamp = (unsigned long)time(NULL);
    unsigned int pid = 0;
#ifdef _WIN32
    pid = GetCurrentProcessId();
#else
    pid = getpid();
#endif
    
    size_t len = strlen(tmp_dir) + strlen(prefix) + 50;
    char* temp_path = (char*)malloc(len);
    if (!temp_path) {
        free(tmp_dir);
        return NULL;
    }
    
#ifdef _WIN32
    snprintf(temp_path, len, "%s\\%s_%lu_%u", tmp_dir, prefix, timestamp, pid);
#else
    snprintf(temp_path, len, "%s/%s_%lu_%u", tmp_dir, prefix, timestamp, pid);
#endif
    
    free(tmp_dir);
    
    FILE* f = fopen(temp_path, "wb");
    if (f) fclose(f);
    
    return temp_path;
}

char* fs_man_create_temp_dir(const char* prefix) {
    if (!prefix) prefix = "tmp";
    
    char* tmp_dir = fs_man_temp_dir();
    if (!tmp_dir) return NULL;
    
    unsigned long timestamp = (unsigned long)time(NULL);
    unsigned int pid = 0;
#ifdef _WIN32
    pid = GetCurrentProcessId();
#else
    pid = getpid();
#endif
    
    size_t len = strlen(tmp_dir) + strlen(prefix) + 50;
    char* temp_path = (char*)malloc(len);
    if (!temp_path) {
        free(tmp_dir);
        return NULL;
    }
    
#ifdef _WIN32
    snprintf(temp_path, len, "%s\\%s_%lu_%u", tmp_dir, prefix, timestamp, pid);
#else
    snprintf(temp_path, len, "%s/%s_%lu_%u", tmp_dir, prefix, timestamp, pid);
#endif
    
    free(tmp_dir);
    
    fs_mkdir(temp_path, 0755);
    
    return temp_path;
}

char* fs_man_expand_tilde(const char* path) {
    if (!path || path[0] != '~') {
        return fs_man_strdup(path);
    }
    
    char* home = fs_man_home_dir();
    if (!home || strlen(home) == 0) {
        free(home);
        return fs_man_strdup(path);
    }
    
    if (strlen(path) == 1) {
        return home;
    }
    
    const char* rest = path + 1;
    if (rest[0] != '/' && rest[0] != '\\') {
        free(home);
        return fs_man_strdup(path);
    }
    
    size_t len = strlen(home) + strlen(rest) + 1;
    char* result = (char*)malloc(len);
    if (!result) {
        free(home);
        return NULL;
    }
    
    strcpy(result, home);
    strcat(result, rest);
    free(home);
    
    return result;
}

void fs_man_copy_dir(const char* src, const char* dst) {
    if (!src || !dst) return;
    fs_man_create_dir_all(dst);
}

StringArray* fs_man_read_dir(const char* path) {
    if (!path) return NULL;
    
#ifdef _WIN32
    WIN32_FIND_DATA find_data;
    char search_path[MAX_PATH];
    snprintf(search_path, MAX_PATH, "%s\\*", path);
    
    HANDLE h_find = FindFirstFile(search_path, &find_data);
    if (h_find == INVALID_HANDLE_VALUE) return NULL;
    
    StringArray* result = (StringArray*)malloc(sizeof(StringArray));
    if (!result) {
        FindClose(h_find);
        return NULL;
    }
    
    result->items = NULL;
    result->count = 0;
    size_t capacity = 0;
    
    do {
        if (strcmp(find_data.cFileName, ".") == 0 || strcmp(find_data.cFileName, "..") == 0) {
            continue;
        }
        
        if (result->count >= capacity) {
            capacity = capacity == 0 ? 16 : capacity * 2;
            char** new_items = (char**)realloc(result->items, capacity * sizeof(char*));
            if (!new_items) {
                for (size_t i = 0; i < result->count; i++) {
                    free(result->items[i]);
                }
                free(result->items);
                free(result);
                FindClose(h_find);
                return NULL;
            }
            result->items = new_items;
        }
        
        result->items[result->count] = fs_man_strdup(find_data.cFileName);
        result->count++;
    } while (FindNextFile(h_find, &find_data));
    
    FindClose(h_find);
    return result;
    
#else
    DIR* dir = opendir(path);
    if (!dir) return NULL;
    
    StringArray* result = (StringArray*)malloc(sizeof(StringArray));
    if (!result) {
        closedir(dir);
        return NULL;
    }
    
    result->items = NULL;
    result->count = 0;
    size_t capacity = 0;
    
    struct dirent* entry;
    while ((entry = readdir(dir)) != NULL) {
        if (strcmp(entry->d_name, ".") == 0 || strcmp(entry->d_name, "..") == 0) {
            continue;
        }
        
        if (result->count >= capacity) {
            capacity = capacity == 0 ? 16 : capacity * 2;
            char** new_items = (char**)realloc(result->items, capacity * sizeof(char*));
            if (!new_items) {
                for (size_t i = 0; i < result->count; i++) {
                    free(result->items[i]);
                }
                free(result->items);
                free(result);
                closedir(dir);
                return NULL;
            }
            result->items = new_items;
        }
        
        result->items[result->count] = fs_man_strdup(entry->d_name);
        result->count++;
    }
    
    closedir(dir);
    return result;
#endif
}