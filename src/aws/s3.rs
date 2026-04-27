use anyhow::anyhow;
use aws_sdk_s3::error::SdkError;
use log::debug;

/// Helper functions for S3 error handling
pub struct S3ErrorHelper;

impl S3ErrorHelper {
    pub fn extract_error_details<E>(error: &SdkError<E>) -> String 
    where 
        E: std::fmt::Debug + std::fmt::Display
    {
        debug!("Extracting error details from AWS SDK error: {:?}", error);
        
        let error_string = format!("{:?}", error);
        
        let error_type = if error_string.contains("AccessDenied") {
            "AccessDenied"
        } else if error_string.contains("NoSuchBucket") {
            "NoSuchBucket"
        } else if error_string.contains("InvalidAccessKeyId") {
            "InvalidAccessKeyId"
        } else if error_string.contains("SignatureDoesNotMatch") {
            "SignatureDoesNotMatch"
        } else if error_string.contains("ExpiredToken") {
            "ExpiredToken"
        } else if error_string.contains("InvalidToken") {
            "InvalidToken"
        } else if error_string.contains("AuthorizationHeaderMalformed") {
            "AuthorizationHeaderMalformed"
        } else {
            "Unknown"
        };
        
        let additional_info = match error_type {
            "AccessDenied" => " - Check your IAM permissions for this bucket",
            "NoSuchBucket" => " - The specified bucket does not exist",
            "InvalidAccessKeyId" => " - The AWS access key ID you provided does not exist",
            "SignatureDoesNotMatch" => " - The signature calculation is incorrect, check your secret key",
            "ExpiredToken" => " - The provided token has expired, please refresh your credentials",
            "InvalidToken" => " - The provided token is invalid, please check your credentials",
            "AuthorizationHeaderMalformed" => " - The authorization header is malformed, check region configuration",
            _ => "",
        };
        
        format!(
            "AWS S3 error - Type: {}, Raw: {}{}", 
            error_type,
            error,
            additional_info
        )
    }
    
    #[allow(dead_code)]
    pub fn convert_sdk_error<E>(error: SdkError<E>, operation: &str) -> anyhow::Error 
    where 
        E: std::fmt::Debug + std::fmt::Display
    {
        let detailed_error = Self::extract_error_details(&error);
        anyhow!("S3 {} operation failed: {}", operation, detailed_error)
    }
}
