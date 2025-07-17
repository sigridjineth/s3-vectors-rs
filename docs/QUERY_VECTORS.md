English
Preferences
Contact Us
Feedback
AWS Documentation

Get started
Service guides
Developer tools
AI resources

Return to the Console

Documentation
Amazon Simple Storage Service (S3)
API Reference
Documentation
Amazon Simple Storage Service (S3)
API Reference
QueryVectors
PDF
Focus mode
Note
Amazon S3 Vectors is in preview release for Amazon S3 and is subject to change.

Performs an approximate nearest neighbor search query in a vector index using a query vector. By default, it returns the keys of approximate nearest neighbors. You can optionally include the computed distance (between the query vector and each vector in the response), the vector data, and metadata of each vector in the response.

To specify the vector index, you can either use both the vector bucket name and the vector index name, or use the vector index Amazon Resource Name (ARN).

Permissions
You must have the s3vectors:QueryVectors permission to use this operation. Additional permissions are required based on the request parameters you specify:

With only s3vectors:QueryVectors permission, you can retrieve vector keys of approximate nearest neighbors and computed distances between these vectors. This permission is sufficient only when you don't set any metadata filters and don't request vector data or metadata (by keeping the returnMetadata parameter set to false or not specified).

If you specify a metadata filter or set returnMetadata to true, you must have both s3vectors:QueryVectors and s3vectors:GetVectors permissions. The request fails with a 403 Forbidden error if you request metadata filtering, vector data, or metadata without the s3vectors:GetVectors permission.

Request Syntax

POST /QueryVectors HTTP/1.1
Content-type: application/json

{
"filter": JSON value,
"indexArn": "string",
"indexName": "string",
"queryVector": { ... },
"returnDistance": boolean,
"returnMetadata": boolean,
"topK": number,
"vectorBucketName": "string"
}
URI Request Parameters

The request does not use any URI parameters.

Request Body

The request accepts the following data in JSON format.

filter
Metadata filter to apply during the query. For more information about metadata keys, see Metadata filtering in the Amazon S3 User Guide.

Type: JSON value

Required: No

indexArn
The ARN of the vector index that you want to query.

Type: String

Required: No

indexName
The name of the vector index that you want to query.

Type: String

Length Constraints: Minimum length of 3. Maximum length of 63.

Required: No

queryVector
The query vector. Ensure that the query vector has the same dimension as the dimension of the vector index that's being queried. For example, if your vector index contains vectors with 384 dimensions, your query vector must also have 384 dimensions.

Type: VectorData object

Note: This object is a Union. Only one member of this object can be specified or returned.

Required: Yes

returnDistance
Indicates whether to include the computed distance in the response. The default value is false.

Type: Boolean

Required: No

returnMetadata
Indicates whether to include metadata in the response. The default value is false.

Type: Boolean

Required: No

topK
The number of results to return for each query.

Type: Integer

Valid Range: Minimum value of 1.

Required: Yes

vectorBucketName
The name of the vector bucket that contains the vector index.

Type: String

Length Constraints: Minimum length of 3. Maximum length of 63.

Required: No

Response Syntax

HTTP/1.1 200
Content-type: application/json

{
"vectors": [
{
"data": { ... },
"distance": number,
"key": "string",
"metadata": JSON value
}
]
}
Response Elements

If the action is successful, the service sends back an HTTP 200 response.

The following data is returned in JSON format by the service.

vectors
The vectors in the approximate nearest neighbor search.

Type: Array of QueryOutputVector objects

Errors

AccessDeniedException
Access denied.

HTTP Status Code: 403

InternalServerException
The request failed due to an internal server error.

HTTP Status Code: 500

KmsDisabledException
The specified AWS KMS key isn't enabled.

HTTP Status Code: 400

KmsInvalidKeyUsageException
The request was rejected for one of the following reasons:

The KeyUsage value of the KMS key is incompatible with the API operation.

The encryption algorithm or signing algorithm specified for the operation is incompatible with the type of key material in the KMS key (KeySpec).

For more information, see InvalidKeyUsageException in the AWS Key Management Service API Reference.

HTTP Status Code: 400

KmsInvalidStateException
The key state of the KMS key isn't compatible with the operation.

For more information, see KMSInvalidStateException in the AWS Key Management Service API Reference.

HTTP Status Code: 400

KmsNotFoundException
The KMS key can't be found.

HTTP Status Code: 400

NotFoundException
The request was rejected because the specified resource can't be found.

HTTP Status Code: 404

ServiceQuotaExceededException
Your request exceeds a service quota.

HTTP Status Code: 402

ServiceUnavailableException
The service is unavailable. Wait briefly and retry your request. If it continues to fail, increase your waiting time between retries.

HTTP Status Code: 503

TooManyRequestsException
The request was denied due to request throttling.

HTTP Status Code: 429

ValidationException
The requested action isn't valid.

HTTP Status Code: 400

See Also

For more information about using this API in one of the language-specific AWS SDKs, see the following:

AWS Command Line Interface

AWS SDK for .NET

AWS SDK for C++

AWS SDK for Go v2

AWS SDK for Java V2

AWS SDK for JavaScript V3

AWS SDK for Kotlin

AWS SDK for PHP V3

AWS SDK for Python

AWS SDK for Ruby V3

Discover highly rated pages Abstracts generated by AI

1
2
3
4

AmazonS3 › userguide
What is Amazon S3?
Amazon S3 enables storing, managing, and accessing data objects, with features for optimizing storage costs, ensuring data consistency, and controlling access.
July 16, 2025
AmazonS3 › userguide
General purpose bucket naming rules
Bucket naming rules include length, valid characters, formatting, uniqueness. Avoid periods, choose relevant names, include GUIDs. Create buckets with GUIDs using AWS CLI, SDK.
July 16, 2025
AmazonS3 › userguide
Hosting a static website using Amazon S3
Enabling website hosting on Amazon S3 allows hosting static websites with static content and client-side scripts. Configure index document, custom error document, permissions, logging, redirects, and cross-origin resource sharing.
February 26, 2025

On this page
Request Syntax
URI Request Parameters
Request Body
Response Syntax
Response Elements
Errors
See Also
Did this page help you?
Yes
No
Provide feedback

Next topic:Data Types
Previous topic:PutVectors
Need help?
Try AWS re:Post
Connect with an AWS IQ expert
PrivacySite termsCookie preferences© 2025, Amazon Web Services, Inc. or its affiliates. All rights reserved.