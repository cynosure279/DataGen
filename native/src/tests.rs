// Test module for datagen-native

#[cfg(test)]
mod tests {
    use crate::compile::{CompileError, CompileRunner};

    fn find_cxx_compiler() -> Option<String> {
        for candidate in &["g++", "clang++", "c++"] {
            if which::which(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
        None
    }

    #[tokio::test]
    async fn test_valid_cpp_compilation() {
        let compiler = find_cxx_compiler().expect("No C++ compiler found on system");
        let runner = CompileRunner;
        let source = "int main() { return 0; }";
        let result = runner
            .compile(source, "cpp", &compiler, &[])
            .await
            .expect("Compilation should succeed");

        assert!(result.success, "Expected compilation success");
        assert!(result.binary_path.is_some(), "Expected a binary path");
        assert_eq!(result.exit_code, Some(0), "Expected exit code 0");
    }

    #[tokio::test]
    async fn test_syntax_error_cpp() {
        let compiler = find_cxx_compiler().expect("No C++ compiler found on system");
        let runner = CompileRunner;
        let source = "int main() { this is not valid c++ }";
        let result = runner
            .compile(source, "cpp", &compiler, &[])
            .await
            .expect("Compilation should return a result (not error)");

        assert!(!result.success, "Expected compilation failure");
        assert!(result.binary_path.is_none(), "Expected no binary path");
        assert!(!result.stderr.is_empty(), "Expected stderr output");
        assert_ne!(result.exit_code, Some(0), "Expected non-zero exit code");
    }

    #[tokio::test]
    async fn test_python_no_compilation() {
        let runner = CompileRunner;
        let result = runner
            .compile("print('hello')", "python", "g++", &[])
            .await
            .expect("Python should return success without compilation");

        assert!(result.success, "Expected success for Python");
        assert!(result.binary_path.is_none(), "Expected no binary path for Python");
        assert_eq!(result.exit_code, Some(0), "Expected exit code 0");
    }

    #[tokio::test]
    async fn test_python_case_insensitive() {
        let runner = CompileRunner;
        let result = runner
            .compile("x = 1", "Python", "g++", &[])
            .await
            .expect("Python (case-insensitive) should succeed");

        assert!(result.success);
        assert!(result.binary_path.is_none());
    }

    #[tokio::test]
    async fn test_compiler_not_found() {
        let runner = CompileRunner;
        let err = runner
            .compile("int main() {}", "cpp", "/nonexistent/compiler", &[])
            .await
            .expect_err("Expected CompilerNotFound error");

        match err {
            CompileError::CompilerNotFound(ref path) => {
                assert_eq!(path, "/nonexistent/compiler");
            }
            _ => panic!("Expected CompilerNotFound, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_c_compilation() {
        let compiler = find_cxx_compiler().expect("No C++ compiler found on system");
        let runner = CompileRunner;
        let source = "int main() { return 0; }";
        let result = runner
            .compile(source, "c", &compiler, &[])
            .await
            .expect("C compilation should succeed");

        assert!(result.success);
        assert!(result.binary_path.is_some());
    }

    #[tokio::test]
    async fn test_compile_with_extra_args() {
        let compiler = find_cxx_compiler().expect("No C++ compiler found on system");
        let runner = CompileRunner;
        let source = r#"
#include <iostream>
int main() { std::cout << "hello"; return 0; }
"#;
        let args = vec!["-std=c++17".to_string(), "-O2".to_string()];
        let result = runner
            .compile(source, "cpp", &compiler, &args)
            .await
            .expect("Compilation with args should succeed");

        assert!(result.success);
        assert!(result.binary_path.is_some());
    }
}