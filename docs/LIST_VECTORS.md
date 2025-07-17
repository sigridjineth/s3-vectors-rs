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
ListVectors
PDF
Focus mode
Note
Amazon S3 Vectors is in preview release for Amazon S3 and is subject to change.

List vectors in the specified vector index. To specify the vector index, you can either use both the vector bucket name and the vector index name, or use the vector index Amazon Resource Name (ARN).

ListVectors operations proceed sequentially; however, for faster performance on a large number of vectors in a vector index, applications can request a parallel ListVectors operation by providing the segmentCount and segmentIndex parameters.

Permissions
You must have the s3vectors:ListVectors permission to use this operation. Additional permissions are required based on the request parameters you specify:

With only s3vectors:ListVectors permission, you can list vector keys when returnData and returnMetadata are both set to false or not specified..

If you set returnData or returnMetadata to true, you must have both s3vectors:ListVectors and s3vectors:GetVectors permissions. The request fails with a 403 Forbidden error if you request vector data or metadata without the s3vectors:GetVectors permission.

Request Syntax

POST /ListVectors HTTP/1.1
Content-type: application/json

{
"indexArn": "string",
"indexName": "string",
"maxResults": number,
"nextToken": "string",
"returnData": boolean,
"returnMetadata": boolean,
"segmentCount": number,
"segmentIndex": number,
"vectorBucketName": "string"
}
URI Request Parameters

The request does not use any URI parameters.

Request Body

The request accepts the following data in JSON format.

indexArn
The Amazon resource Name (ARN) of the vector index.

Type: String

Required: No

indexName
The name of the vector index.

Type: String

Length Constraints: Minimum length of 3. Maximum length of 63.

Required: No

maxResults
The maximum number of vectors to return on a page.

If you don't specify maxResults, the ListVectors operation uses a default value of 500.

If the processed dataset size exceeds 1 MB before reaching the maxResults value, the operation stops and returns the vectors that are retrieved up to that point, along with a nextToken that you can use in a subsequent request to retrieve the next set of results.

Type: Integer

Valid Range: Minimum value of 1. Maximum value of 1000.

Required: No

nextToken
Pagination token from a previous request. The value of this field is empty for an initial request.

Type: String

Length Constraints: Minimum length of 1. Maximum length of 2048.

Required: No

returnData
If true, the vector data of each vector will be included in the response. The default value is false.

Type: Boolean

Required: No

returnMetadata
If true, the metadata associated with each vector will be included in the response. The default value is false.

Type: Boolean

Required: No

segmentCount
For a parallel ListVectors request, segmentCount represents the total number of vector segments into which the ListVectors operation will be divided. The value of segmentCount corresponds to the number of application workers that will perform the parallel ListVectors operation. For example, if you want to use four application threads to list vectors in a vector index, specify a segmentCount value of 4.

If you specify a segmentCount value of 1, the ListVectors operation will be sequential rather than parallel.

If you specify segmentCount, you must also specify segmentIndex.

Type: Integer

Valid Range: Minimum value of 1. Maximum value of 16.

Required: No

segmentIndex
For a parallel ListVectors request, segmentIndex is the index of the segment from which to list vectors in the current request. It identifies an individual segment to be listed by an application worker.

Segment IDs are zero-based, so the first segment is always 0. For example, if you want to use four application threads to list vectors in a vector index, then the first thread specifies a segmentIndex value of 0, the second thread specifies 1, and so on.

The value of segmentIndex must be less than the value provided for segmentCount.

If you provide segmentIndex, you must also provide segmentCount.

Type: Integer

Valid Range: Minimum value of 0. Maximum value of 15.

Required: No

vectorBucketName
The name of the vector bucket.

Type: String

Length Constraints: Minimum length of 3. Maximum length of 63.

Required: No

Response Syntax

HTTP/1.1 200
Content-type: application/json

{
"nextToken": "string",
"vectors": [
{
"data": { ... },
"key": "string",
"metadata": JSON value
}
]
}
Response Elements

If the action is successful, the service sends back an HTTP 200 response.

The following data is returned in JSON format by the service.

nextToken
Pagination token to be used in the subsequent request. The field is empty if no further pagination is required.

Type: String

Length Constraints: Minimum length of 1. Maximum length of 2048.

vectors
Vectors in the current segment.

Type: Array of ListOutputVector objects

Errors

AccessDeniedException
Access denied.

HTTP Status Code: 403

InternalServerException
The request failed due to an internal server error.

HTTP Status Code: 500

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

Next topic:PutVectorBucketPolicy
Previous topic:ListVectorBuckets
Need help?
Try AWS re:Post
Connect with an AWS IQ expert
PrivacySite termsCookie preferences© 2025, Amazon Web Services, Inc. or its affiliates. All rights reserved.