(module
  ;; Simple test function that adds two i32 numbers
  (func $add (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.add)

  ;; Simple test function with no parameters
  (func $test (result i32)
    i32.const 42)

  ;; Export functions
  (export "add" (func $add))
  (export "test" (func $test))
)
