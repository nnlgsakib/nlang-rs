mod path_utils;
mod metadata;
mod file_io;
mod file_ops;
mod dir_ops;
mod links;
mod env_paths;

use crate::ast::Type;
use crate::nlang_libs::common::LibraryDefinition;

pub fn create_fs_man_lib() -> LibraryDefinition {
    let mut lib = LibraryDefinition::new("fs_man");
    
    lib.add_function("exists", vec![Type::String], Type::Boolean, path_utils::exists);
    lib.add_function("is_file", vec![Type::String], Type::Boolean, path_utils::is_file);
    lib.add_function("is_dir", vec![Type::String], Type::Boolean, path_utils::is_dir);
    lib.add_function("path_join", vec![Type::String, Type::String], Type::String, path_utils::join);
    lib.add_function("basename", vec![Type::String], Type::String, path_utils::basename);
    lib.add_function("dirname", vec![Type::String], Type::String, path_utils::dirname);
    lib.add_function("extname", vec![Type::String], Type::String, path_utils::extname);
    lib.add_function("create_path", vec![Type::String], Type::Void, path_utils::create_path);
    lib.add_function("remove_path", vec![Type::String], Type::Void, path_utils::remove_path);
    
    lib.add_function("file_stat", vec![Type::String], Type::Array(Box::new(Type::Integer), 5), metadata::stat);
    lib.add_function("file_size", vec![Type::String], Type::Integer, metadata::size);
    lib.add_function("last_modified", vec![Type::String], Type::Float, metadata::last_modified);
    lib.add_function("permissions", vec![Type::String], Type::Integer, metadata::permissions);
    lib.add_function("set_permissions", vec![Type::String, Type::Integer], Type::Void, metadata::set_permissions);
    
    lib.add_function("read_file", vec![Type::String], Type::String, file_io::read_file);
    lib.add_function("read_bytes", vec![Type::String], Type::Array(Box::new(Type::Integer), 0), file_io::read_bytes);
    lib.add_function("write_file", vec![Type::String, Type::String], Type::Void, file_io::write_file);
    lib.add_function("write_bytes", vec![Type::String, Type::Array(Box::new(Type::Integer), 0)], Type::Void, file_io::write_bytes);
    lib.add_function("append_file", vec![Type::String, Type::String], Type::Void, file_io::append_file);
    lib.add_function("read_lines", vec![Type::String], Type::Array(Box::new(Type::String), 0), file_io::read_lines);
    lib.add_function("truncate_file", vec![Type::String, Type::Integer], Type::Void, file_io::truncate_file);
    
    lib.add_function("copy_file", vec![Type::String, Type::String], Type::Void, file_ops::copy_file);
    lib.add_function("move_file", vec![Type::String, Type::String], Type::Void, file_ops::move_file);
    lib.add_function("rename_file", vec![Type::String, Type::String], Type::Void, file_ops::rename_file);
    lib.add_function("copy_dir", vec![Type::String, Type::String], Type::Void, file_ops::copy_dir);
    lib.add_function("create_file", vec![Type::String], Type::Void, file_ops::create_file);
    lib.add_function("remove_file", vec![Type::String], Type::Void, file_ops::remove_file);
    lib.add_function("remove_dir", vec![Type::String], Type::Void, file_ops::remove_dir);
    lib.add_function("remove_dir_all", vec![Type::String], Type::Void, file_ops::remove_dir_all);
    
    lib.add_function("read_dir", vec![Type::String], Type::Array(Box::new(Type::String), 0), dir_ops::read_dir);
    lib.add_function("list_files", vec![Type::String], Type::Array(Box::new(Type::String), 0), dir_ops::list_files);
    lib.add_function("list_dirs", vec![Type::String], Type::Array(Box::new(Type::String), 0), dir_ops::list_dirs);
    lib.add_function("walk_dir", vec![Type::String], Type::Array(Box::new(Type::String), 0), dir_ops::walk_dir);
    lib.add_function("glob_pattern", vec![Type::String, Type::String], Type::Array(Box::new(Type::String), 0), dir_ops::glob_pattern);
    lib.add_function("create_dir", vec![Type::String], Type::Void, dir_ops::create_dir);
    lib.add_function("create_dir_all", vec![Type::String], Type::Void, dir_ops::create_dir_all);
    
    lib.add_function("is_symlink", vec![Type::String], Type::Boolean, links::is_symlink);
    lib.add_function("read_link", vec![Type::String], Type::String, links::read_link);
    lib.add_function("create_symlink", vec![Type::String, Type::String], Type::Void, links::create_symlink);
    lib.add_function("create_hard_link", vec![Type::String, Type::String], Type::Void, links::create_hard_link);
    lib.add_function("canonicalize", vec![Type::String], Type::String, links::canonicalize);
    lib.add_function("absolute_path", vec![Type::String], Type::String, links::absolute_path);
    lib.add_function("is_absolute", vec![Type::String], Type::Boolean, links::is_absolute);
    lib.add_function("is_relative", vec![Type::String], Type::Boolean, links::is_relative);
    
    lib.add_function("temp_dir", vec![], Type::String, env_paths::temp_dir);
    lib.add_function("current_dir", vec![], Type::String, env_paths::current_dir);
    lib.add_function("set_current_dir", vec![Type::String], Type::Void, env_paths::set_current_dir);
    lib.add_function("home_dir", vec![], Type::String, env_paths::home_dir);
    lib.add_function("create_temp_file", vec![Type::String], Type::String, env_paths::create_temp_file);
    lib.add_function("create_temp_dir", vec![Type::String], Type::String, env_paths::create_temp_dir);
    lib.add_function("get_path_separator", vec![], Type::String, env_paths::get_path_separator);
    lib.add_function("expand_tilde", vec![Type::String], Type::String, env_paths::expand_tilde);
    
    lib.c_implementation = Some(include_str!("fs_man.c").to_string());
    
    // Helper for common string return functions
    let string_ret_funcs = vec![
        ("path_join", "fs_man_join({0}, {1})"),
        ("basename", "fs_man_basename({0})"),
        ("dirname", "fs_man_dirname({0})"),
        ("extname", "fs_man_extname({0})"),
        ("read_file", "fs_man_read_file({0})"),
        ("current_dir", "fs_man_current_dir()"),
        ("temp_dir", "fs_man_temp_dir()"),
        ("home_dir", "fs_man_home_dir()"),
        ("get_path_separator", "fs_man_get_path_separator()"),
        ("read_link", "fs_man_read_link({0})"),
        ("canonicalize", "fs_man_canonicalize({0})"),
        ("absolute_path", "fs_man_absolute_path({0})"),
        ("create_temp_file", "fs_man_create_temp_file({0})"),
        ("create_temp_dir", "fs_man_create_temp_dir({0})"),
        ("expand_tilde", "fs_man_expand_tilde({0})"),
    ];
    
    for func in &mut lib.functions {
        let impl_opt = match func.name.as_str() {
            // Boolean returns
            "exists" => Some("fs_man_exists({0})"),
            "is_file" => Some("fs_man_is_file({0})"),
            "is_dir" => Some("fs_man_is_dir({0})"),
            "is_symlink" => Some("fs_man_is_symlink({0})"),
            "is_absolute" => Some("fs_man_is_absolute({0})"),
            "is_relative" => Some("fs_man_is_relative({0})"),
            
            // Integer returns
            "file_size" => Some("fs_man_file_size({0})"),
            "permissions" => Some("fs_man_permissions({0})"),
            
            // Float returns
            "last_modified" => Some("fs_man_last_modified({0})"),
            
            // Void returns
            "create_path" => Some("fs_man_create_path({0})"),
            "remove_path" => Some("fs_man_remove_path({0})"),
            "set_permissions" => Some("fs_man_set_permissions({0}, {1})"),
            "write_file" => Some("fs_man_write_file({0}, {1})"),
            "append_file" => Some("fs_man_append_file({0}, {1})"),
            "copy_file" => Some("fs_man_copy_file({0}, {1})"),
            "move_file" => Some("fs_man_move_file({0}, {1})"),
            "rename_file" => Some("fs_man_rename_file({0}, {1})"),
            "create_file" => Some("fs_man_create_file({0})"),
            "remove_file" => Some("fs_man_remove_file({0})"),
            "remove_dir" => Some("fs_man_remove_dir({0})"),
            "remove_dir_all" => Some("fs_man_remove_dir_all({0})"),
            "create_dir" => Some("fs_man_create_dir({0})"),
            "create_dir_all" => Some("fs_man_create_dir_all({0})"),
            "set_current_dir" => Some("fs_man_set_current_dir({0})"),
            "copy_dir" => Some("fs_man_copy_dir({0}, {1})"),
            "create_symlink" => Some("fs_man_create_symlink({0}, {1})"),
            "create_hard_link" => Some("fs_man_create_hard_link({0}, {1})"),
            
            // Special handling for read_dir - returns dynamic string array
            "read_dir" => Some("fs_man_read_dir({0})"),
            
            // String returns - check helper list
            name => string_ret_funcs.iter()
                .find(|(fname, _)| *fname == name)
                .map(|(_, impl_str)| *impl_str),
        };
        
        if let Some(c_impl) = impl_opt {
            func.c_implementation = Some(c_impl.to_string());
        }
    }
    
    lib
}