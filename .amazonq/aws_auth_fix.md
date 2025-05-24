# AWS Authentication Fix Plan

Based on the code analysis, here are the issues and fixes needed:

## 1. Fix Syntax Errors in app.rs

- Add missing semicolons at lines 652 and 769

## 2. Fix Type Mismatches in TransferManager

- In `src/ui/app.rs`, the `auth_clone` variable is of type `()` but `TransferManager::new()` expects `AwsAuth`
- Need to ensure proper `AwsAuth` object is created and passed

## 3. Fix Progress Callback Type Mismatches

- In `src/sync/engine.rs`, the progress callbacks need to be boxed:
  - Change from `Option<&dyn Fn(TransferProgress)>` to `Option<Box<dyn Fn(TransferProgress) + Send + Sync>>`
  - Or modify the function signatures to accept references

## 4. Fix BucketView.load_objects Type Mismatch

- In `src/ui/app.rs`, the `auth` variable is of type `Arc<Mutex<()>>` but `load_objects` expects `Arc<Mutex<AwsAuth>>`
- Need to ensure proper `AwsAuth` object is created and wrapped in Arc<Mutex<>>

## Implementation Plan

1. First, fix the syntax errors (missing semicolons)
2. Create proper AwsAuth instances in app.rs
3. Fix the progress callback type mismatches
4. Ensure proper Arc<Mutex<AwsAuth>> is passed to load_objects
