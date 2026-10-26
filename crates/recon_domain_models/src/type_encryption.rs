use async_trait::async_trait;
use common_utils::{
    crypto,
    encryption::Encryption,
    errors::{self, CustomResult},
};
use masking::Secret;
use rustc_hash::FxHashMap;

pub mod encrypt {
    use async_trait::async_trait;
    use common_utils::{
        chunking::chunked,
        crypto,
        encryption::Encryption,
        errors::{self, CustomResult},
        ext_traits::ByteSliceExt,
        keymanager::call_encryption_service,
        transformers::{ForeignFrom, ForeignTryFrom},
        types::keymanager::{
            BatchDecryptDataResponse, BatchEncryptDataRequest, BatchEncryptDataResponse,
            DecryptDataResponse, EncryptDataRequest, EncryptDataResponse, Identifier,
            KeyManagerState, TransientBatchDecryptDataRequest, TransientDecryptDataRequest,
        },
    };
    use error_stack::ResultExt;
    use futures::{StreamExt, TryStreamExt};
    use masking::{PeekInterface, Secret};
    use reqwest::Method;
    use rustc_hash::FxHashMap;
    use tracing::instrument;

    use super::EncryptedJsonType;

    const BATCH_ENCRYPTION_CHUNK_SIZE: usize = 500;
    const BATCH_ENCRYPTION_MAX_CONCURRENT: usize = 4;
    const BATCH_DECRYPTION_CHUNK_SIZE: usize = 500;
    const BATCH_DECRYPTION_MAX_CONCURRENT: usize = 4;

    async fn chunked_batch_encrypt<Out, V, S>(
        state: &KeyManagerState,
        masked_data: FxHashMap<String, Secret<V, S>>,
        identifier: Identifier,
    ) -> CustomResult<FxHashMap<String, Out>, errors::CryptoError>
    where
        Out: Send,
        V: Clone + Send,
        S: masking::Strategy<V> + Send + Sync,
        Secret<V, S>: Send,
        BatchEncryptDataRequest: From<(FxHashMap<String, Secret<V, S>>, Identifier)>,
        FxHashMap<String, Out>:
            ForeignFrom<(FxHashMap<String, Secret<V, S>>, BatchEncryptDataResponse)>,
    {
        let capacity = masked_data.len();
        let chunks = chunked(masked_data, BATCH_ENCRYPTION_CHUNK_SIZE)
            .map(|chunk| chunk.into_iter().collect::<FxHashMap<_, _>>());

        futures::stream::iter(chunks)
            .map(|chunk| {
                let identifier = identifier.clone();
                async move {
                    let response = call_encryption_service::<_, BatchEncryptDataResponse>(
                        state,
                        Method::POST,
                        "data/encrypt",
                        BatchEncryptDataRequest::from((chunk.clone(), identifier)),
                    )
                    .await
                    .change_context(errors::CryptoError::EncodingFailed)?;
                    Ok(ForeignFrom::foreign_from((chunk, response)))
                }
            })
            .buffer_unordered(BATCH_ENCRYPTION_MAX_CONCURRENT)
            .try_fold(
                FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
                |mut acc, batch: FxHashMap<String, Out>| async move {
                    acc.extend(batch);
                    Ok(acc)
                },
            )
            .await
    }

    async fn chunked_batch_decrypt<Out>(
        state: &KeyManagerState,
        encrypted_data: FxHashMap<String, Encryption>,
        identifier: Identifier,
    ) -> CustomResult<FxHashMap<String, Out>, errors::CryptoError>
    where
        Out: Send,
        FxHashMap<String, Out>: ForeignTryFrom<
            (FxHashMap<String, Encryption>, BatchDecryptDataResponse),
            Error = error_stack::Report<errors::CryptoError>,
        >,
    {
        let capacity = encrypted_data.len();
        let chunks = chunked(encrypted_data, BATCH_DECRYPTION_CHUNK_SIZE)
            .map(|chunk| chunk.into_iter().collect::<FxHashMap<_, _>>());

        futures::stream::iter(chunks)
            .map(|chunk| {
                let identifier = identifier.clone();
                async move {
                    let response = call_encryption_service::<_, BatchDecryptDataResponse>(
                        state,
                        Method::POST,
                        "data/decrypt",
                        TransientBatchDecryptDataRequest::from((chunk.clone(), identifier)),
                    )
                    .await
                    .change_context(errors::CryptoError::DecodingFailed)?;
                    <FxHashMap<String, Out> as ForeignTryFrom<(
                        FxHashMap<String, Encryption>,
                        BatchDecryptDataResponse,
                    )>>::foreign_try_from((chunk, response))
                    .change_context(errors::CryptoError::DecodingFailed)
                }
            })
            .buffer_unordered(BATCH_DECRYPTION_MAX_CONCURRENT)
            .try_fold(
                FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
                |mut acc, batch: FxHashMap<String, Out>| async move {
                    acc.extend(batch);
                    Ok(acc)
                },
            )
            .await
    }

    #[async_trait]
    pub trait TypeEncryption<
        T,
        V: crypto::EncodeMessage + crypto::DecodeMessage,
        S: masking::Strategy<T>,
    >: Sized
    {
        async fn encrypt_via_api(
            state: &KeyManagerState,
            masked_data: Secret<T, S>,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError>;

        async fn decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: Encryption,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError>;

        async fn encrypt(
            masked_data: Secret<T, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError>;

        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError>;

        async fn batch_encrypt_chunked_via_api(
            state: &KeyManagerState,
            masked_data: FxHashMap<String, Secret<T, S>>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

        async fn batch_decrypt_chunked_via_api(
            state: &KeyManagerState,
            encrypted_data: FxHashMap<String, Encryption>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<T, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;

        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError>;
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<String> + Send + Sync,
        > TypeEncryption<String, V, S> for crypto::Encryptable<Secret<String, S>>
    {
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt_via_api(
            state: &KeyManagerState,
            masked_data: Secret<String, S>,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                Method::POST,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            result
                .map(|response| ForeignFrom::foreign_from((masked_data.clone(), response)))
                .change_context(errors::CryptoError::EncodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: Encryption,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                Method::POST,
                "data/decrypt",
                TransientDecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(decrypted_data) => {
                    crypto::Encryptable::foreign_try_from((encrypted_data.clone(), decrypted_data))
                        .change_context(errors::CryptoError::DecodingFailed)
                }
                Err(err) => Err(err.change_context(errors::CryptoError::DecodingFailed)),
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<String, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let encrypted_data = crypt_algo.encode_message(key, masked_data.peek().as_bytes())?;
            // For local encryption, format as versioned base64
            let formatted_data = Encryption::from_raw_bytes(encrypted_data);
            Ok(Self::new(masked_data, formatted_data.into_inner()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            // Extract raw bytes from versioned format if needed
            let raw_bytes = encrypted_data.to_raw_bytes().map_err(|err| {
                error_stack::Report::new(errors::CryptoError::DecodingFailed)
                    .attach_printable(format!("Failed to extract raw bytes: {err}"))
            })?;

            let encrypted_secret = Secret::new(raw_bytes);
            let data = crypt_algo.decode_message(key, encrypted_secret.clone())?;

            let value: String = std::str::from_utf8(&data)
                .change_context(errors::CryptoError::DecodingFailed)?
                .to_string();

            Ok(Self::new(value.into(), encrypted_data.into_inner()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt_chunked_via_api(
            state: &KeyManagerState,
            masked_data: FxHashMap<String, Secret<String, S>>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            chunked_batch_encrypt(state, masked_data, identifier).await
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt_chunked_via_api(
            state: &KeyManagerState,
            encrypted_data: FxHashMap<String, Encryption>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            chunked_batch_decrypt(state, encrypted_data, identifier).await
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<String, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            masked_data
                .into_iter()
                .map(|(k, v)| {
                    let encrypted_data = crypt_algo.encode_message(key, v.peek().as_bytes())?;
                    // For local encryption, format as versioned base64
                    let formatted_data = Encryption::from_raw_bytes(encrypted_data);
                    Ok((k, Self::new(v.clone(), formatted_data.into_inner())))
                })
                .collect()
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            encrypted_data
                .into_iter()
                .map(|(k, v)| {
                    // Extract raw bytes from versioned format
                    let raw_bytes = v.to_raw_bytes().map_err(|err| {
                        error_stack::Report::new(errors::CryptoError::DecodingFailed)
                            .attach_printable(format!("Failed to extract raw bytes: {err}"))
                    })?;

                    let encrypted_secret = Secret::new(raw_bytes);
                    let data = crypt_algo.decode_message(key, encrypted_secret)?;

                    let value: String = std::str::from_utf8(&data)
                        .change_context(errors::CryptoError::DecodingFailed)?
                        .to_string();
                    Ok((k, Self::new(value.into(), v.into_inner())))
                })
                .collect()
        }
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<serde_json::Value> + Send + Sync,
        > TypeEncryption<serde_json::Value, V, S>
        for crypto::Encryptable<Secret<serde_json::Value, S>>
    {
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt_via_api(
            state: &KeyManagerState,
            masked_data: Secret<serde_json::Value, S>,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                Method::POST,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            result
                .map(|response| ForeignFrom::foreign_from((masked_data.clone(), response)))
                .change_context(errors::CryptoError::EncodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: Encryption,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                Method::POST,
                "data/decrypt",
                TransientDecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(decrypted_data) => {
                    crypto::Encryptable::foreign_try_from((encrypted_data.clone(), decrypted_data))
                        .change_context(errors::CryptoError::DecodingFailed)
                }
                Err(err) => Err(err.change_context(errors::CryptoError::DecodingFailed)),
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<serde_json::Value, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let data = serde_json::to_vec(&masked_data.peek())
                .change_context(errors::CryptoError::DecodingFailed)?;
            let encrypted_data = crypt_algo.encode_message(key, &data)?;
            // For local encryption, format as versioned base64
            let formatted_data = Encryption::from_raw_bytes(encrypted_data);
            Ok(Self::new(masked_data, formatted_data.into_inner()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            // Extract raw bytes from versioned format
            let raw_bytes = encrypted_data.to_raw_bytes().map_err(|err| {
                error_stack::Report::new(errors::CryptoError::DecodingFailed)
                    .attach_printable(format!("Failed to extract raw bytes: {err}"))
            })?;

            let encrypted_secret = Secret::new(raw_bytes);
            let data = crypt_algo.decode_message(key, encrypted_secret.clone())?;

            let value: serde_json::Value = serde_json::from_slice(&data)
                .change_context(errors::CryptoError::DecodingFailed)?;
            Ok(Self::new(value.into(), encrypted_data.into_inner()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt_chunked_via_api(
            state: &KeyManagerState,
            masked_data: FxHashMap<String, Secret<serde_json::Value, S>>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            chunked_batch_encrypt(state, masked_data, identifier).await
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt_chunked_via_api(
            state: &KeyManagerState,
            encrypted_data: FxHashMap<String, Encryption>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            chunked_batch_decrypt(state, encrypted_data, identifier).await
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<serde_json::Value, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            masked_data
                .into_iter()
                .map(|(k, v)| {
                    let data = serde_json::to_vec(v.peek())
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    let encrypted_data = crypt_algo.encode_message(key, &data)?;
                    // For local encryption, format as versioned base64
                    let formatted_data = Encryption::from_raw_bytes(encrypted_data);
                    Ok((k, Self::new(v, formatted_data.into_inner())))
                })
                .collect()
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            encrypted_data
                .into_iter()
                .map(|(k, v)| {
                    // Extract raw bytes from versioned format
                    let raw_bytes = v.to_raw_bytes().map_err(|err| {
                        error_stack::Report::new(errors::CryptoError::DecodingFailed)
                            .attach_printable(format!("Failed to extract raw bytes: {err}"))
                    })?;

                    let encrypted_secret = Secret::new(raw_bytes);
                    let data = crypt_algo.decode_message(key, encrypted_secret)?;

                    let value: serde_json::Value = serde_json::from_slice(&data)
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    Ok((k, Self::new(value.into(), v.into_inner())))
                })
                .collect()
        }
    }

    impl<T> EncryptedJsonType<T>
    where
        T: std::fmt::Debug + Clone + serde::Serialize + serde::de::DeserializeOwned,
    {
        fn serialize_json_bytes(&self) -> CustomResult<Secret<Vec<u8>>, errors::CryptoError> {
            common_utils::ext_traits::Encode::encode_to_vec(self.inner())
                .change_context(errors::CryptoError::EncodingFailed)
                .attach_printable("Failed to JSON serialize data before encryption")
                .map(Secret::new)
        }

        fn deserialize_json_bytes<S>(
            bytes: Secret<Vec<u8>>,
        ) -> CustomResult<Secret<Self, S>, errors::ParsingError>
        where
            S: masking::Strategy<Self>,
        {
            bytes
                .peek()
                .as_slice()
                .parse_struct::<T>(std::any::type_name::<T>())
                .map(|result| Secret::new(Self::from(result)))
                .attach_printable("Failed to JSON deserialize data after decryption")
        }
    }

    #[async_trait]
    impl<
            T: std::fmt::Debug + Clone + serde::Serialize + serde::de::DeserializeOwned + Send,
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<EncryptedJsonType<T>> + Send + Sync,
        > TypeEncryption<EncryptedJsonType<T>, V, S>
        for crypto::Encryptable<Secret<EncryptedJsonType<T>, S>>
    {
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt_via_api(
            state: &KeyManagerState,
            masked_data: Secret<EncryptedJsonType<T>, S>,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let data_bytes = EncryptedJsonType::serialize_json_bytes(masked_data.peek())?;
            let result: crypto::Encryptable<Secret<Vec<u8>, masking::WithType>> =
                <crypto::Encryptable<Secret<Vec<u8>, masking::WithType>> as TypeEncryption<
                    Vec<u8>,
                    crypto::GcmAes256,
                    masking::WithType,
                >>::encrypt_via_api(state, data_bytes, identifier)
                .await?;
            Ok(Self::new(masked_data, result.into_encrypted()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: Encryption,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: crypto::Encryptable<Secret<Vec<u8>, masking::WithType>> =
                <crypto::Encryptable<Secret<Vec<u8>, masking::WithType>> as TypeEncryption<
                    Vec<u8>,
                    crypto::GcmAes256,
                    masking::WithType,
                >>::decrypt_via_api(state, encrypted_data, identifier)
                .await?;
            result
                .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                .change_context(errors::CryptoError::DecodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<EncryptedJsonType<T>, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let data_bytes = EncryptedJsonType::serialize_json_bytes(masked_data.peek())?;
            let result: crypto::Encryptable<Secret<Vec<u8>>> =
                TypeEncryption::encrypt(data_bytes, key, crypt_algo).await?;
            Ok(Self::new(masked_data, result.into_encrypted()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: crypto::Encryptable<Secret<Vec<u8>>> =
                TypeEncryption::decrypt(encrypted_data, key, crypt_algo).await?;
            result
                .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                .change_context(errors::CryptoError::DecodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt_chunked_via_api(
            state: &KeyManagerState,
            masked_data: FxHashMap<String, Secret<EncryptedJsonType<T>, S>>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let hashmap_capacity = masked_data.len();
            let data_bytes = masked_data.iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let value_bytes = EncryptedJsonType::serialize_json_bytes(value.peek())?;
                    map.insert(key.to_owned(), value_bytes);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>, masking::WithType>>> =
                <crypto::Encryptable<Secret<Vec<u8>, masking::WithType>> as TypeEncryption<
                    Vec<u8>,
                    crypto::GcmAes256,
                    masking::WithType,
                >>::batch_encrypt_chunked_via_api(state, data_bytes, identifier)
                .await?;
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let original_value = masked_data
                        .get(&key)
                        .ok_or(errors::CryptoError::EncodingFailed)
                        .attach_printable_lazy(|| {
                            format!("Failed to find {key} in input hashmap")
                        })?;
                    map.insert(
                        key,
                        Self::new(original_value.clone(), value.into_encrypted()),
                    );
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt_chunked_via_api(
            state: &KeyManagerState,
            encrypted_data: FxHashMap<String, Encryption>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>, masking::WithType>>> =
                <crypto::Encryptable<Secret<Vec<u8>, masking::WithType>> as TypeEncryption<
                    Vec<u8>,
                    crypto::GcmAes256,
                    masking::WithType,
                >>::batch_decrypt_chunked_via_api(state, encrypted_data, identifier)
                .await?;

            let hashmap_capacity = result.len();
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let deserialized_value = value
                        .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    map.insert(key, deserialized_value);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<EncryptedJsonType<T>, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let hashmap_capacity = masked_data.len();
            let data_bytes = masked_data.iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let value_bytes = EncryptedJsonType::serialize_json_bytes(value.peek())?;
                    map.insert(key.to_owned(), value_bytes);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>>>> =
                TypeEncryption::batch_encrypt(data_bytes, key, crypt_algo).await?;
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let original_value = masked_data
                        .get(&key)
                        .ok_or(errors::CryptoError::EncodingFailed)
                        .attach_printable_lazy(|| {
                            format!("Failed to find {key} in input hashmap")
                        })?;
                    map.insert(
                        key,
                        Self::new(original_value.clone(), value.into_encrypted()),
                    );
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            let result: FxHashMap<String, crypto::Encryptable<Secret<Vec<u8>>>> =
                TypeEncryption::batch_decrypt(encrypted_data, key, crypt_algo).await?;

            let hashmap_capacity = result.len();
            let result_hashmap = result.into_iter().try_fold(
                FxHashMap::with_capacity_and_hasher(hashmap_capacity, Default::default()),
                |mut map, (key, value)| {
                    let deserialized_value = value
                        .deserialize_inner_value(EncryptedJsonType::deserialize_json_bytes)
                        .change_context(errors::CryptoError::DecodingFailed)?;
                    map.insert(key, deserialized_value);
                    Ok::<_, error_stack::Report<errors::CryptoError>>(map)
                },
            )?;

            Ok(result_hashmap)
        }
    }

    #[async_trait]
    impl<
            V: crypto::DecodeMessage + crypto::EncodeMessage + Send + 'static,
            S: masking::Strategy<Vec<u8>> + Send + Sync,
        > TypeEncryption<Vec<u8>, V, S> for crypto::Encryptable<Secret<Vec<u8>, S>>
    {
        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt_via_api(
            state: &KeyManagerState,
            masked_data: Secret<Vec<u8>, S>,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: Result<
                EncryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                Method::POST,
                "data/encrypt",
                EncryptDataRequest::from((masked_data.clone(), identifier)),
            )
            .await;
            result
                .map(|response| ForeignFrom::foreign_from((masked_data.clone(), response)))
                .change_context(errors::CryptoError::EncodingFailed)
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt_via_api(
            state: &KeyManagerState,
            encrypted_data: Encryption,
            identifier: Identifier,
        ) -> CustomResult<Self, errors::CryptoError> {
            let result: Result<
                DecryptDataResponse,
                error_stack::Report<errors::KeyManagerClientError>,
            > = call_encryption_service(
                state,
                Method::POST,
                "data/decrypt",
                TransientDecryptDataRequest::from((encrypted_data.clone(), identifier)),
            )
            .await;
            match result {
                Ok(decrypted_data) => {
                    crypto::Encryptable::foreign_try_from((encrypted_data.clone(), decrypted_data))
                        .change_context(errors::CryptoError::DecodingFailed)
                }
                Err(err) => Err(err.change_context(errors::CryptoError::DecodingFailed)),
            }
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn encrypt(
            masked_data: Secret<Vec<u8>, S>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            let encrypted_data = crypt_algo.encode_message(key, masked_data.peek())?;
            // For local encryption, format as versioned base64
            let formatted_data = Encryption::from_raw_bytes(encrypted_data);
            Ok(Self::new(masked_data, formatted_data.into_inner()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn decrypt(
            encrypted_data: Encryption,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<Self, errors::CryptoError> {
            // Extract raw bytes from versioned format if needed
            let raw_bytes = encrypted_data.to_raw_bytes().map_err(|err| {
                error_stack::Report::new(errors::CryptoError::DecodingFailed)
                    .attach_printable(format!("Failed to extract raw bytes: {err}"))
            })?;

            let encrypted_secret = Secret::new(raw_bytes);
            let data = crypt_algo.decode_message(key, encrypted_secret)?;
            Ok(Self::new(data.into(), encrypted_data.into_inner()))
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt_chunked_via_api(
            state: &KeyManagerState,
            masked_data: FxHashMap<String, Secret<Vec<u8>, S>>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            chunked_batch_encrypt(state, masked_data, identifier).await
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt_chunked_via_api(
            state: &KeyManagerState,
            encrypted_data: FxHashMap<String, Encryption>,
            identifier: Identifier,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            chunked_batch_decrypt(state, encrypted_data, identifier).await
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_encrypt(
            masked_data: FxHashMap<String, Secret<Vec<u8>, S>>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            masked_data
                .into_iter()
                .map(|(k, v)| {
                    let encrypted_data = crypt_algo.encode_message(key, v.peek())?;
                    // For local encryption, format as versioned base64
                    let formatted_data = Encryption::from_raw_bytes(encrypted_data);
                    Ok((k, Self::new(v.clone(), formatted_data.into_inner())))
                })
                .collect()
        }

        // Do not remove the `skip_all` as the key would be logged otherwise
        #[instrument(skip_all)]
        async fn batch_decrypt(
            encrypted_data: FxHashMap<String, Encryption>,
            key: &[u8],
            crypt_algo: V,
        ) -> CustomResult<FxHashMap<String, Self>, errors::CryptoError> {
            encrypted_data
                .into_iter()
                .map(|(k, v)| {
                    // Extract raw bytes from versioned format
                    let raw_bytes = v.to_raw_bytes().map_err(|err| {
                        error_stack::Report::new(errors::CryptoError::DecodingFailed)
                            .attach_printable(format!("Failed to extract raw bytes: {err}"))
                    })?;

                    let encrypted_secret = Secret::new(raw_bytes);
                    let data = crypt_algo.decode_message(key, encrypted_secret)?;
                    Ok((k, Self::new(data.into(), v.into_inner())))
                })
                .collect()
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedJsonType<T>(T);

impl<T> EncryptedJsonType<T> {
    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for EncryptedJsonType<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> std::ops::Deref for EncryptedJsonType<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

/// Type alias for `Option<Encryptable<Secret<EncryptedJsonType<T>>>>`
pub type OptionalEncryptableJsonType<T> = Option<crypto::Encryptable<Secret<EncryptedJsonType<T>>>>;

pub trait Lift<U> {
    type SelfWrapper<T>;
    type OtherWrapper<T, E>;

    fn lift<Func, E, V>(self, func: Func) -> Self::OtherWrapper<V, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> Self::OtherWrapper<V, E>;
}

impl<U> Lift<U> for Option<U> {
    type SelfWrapper<T> = Option<T>;
    type OtherWrapper<T, E> = CustomResult<Option<T>, E>;

    fn lift<Func, E, V>(self, func: Func) -> Self::OtherWrapper<V, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> Self::OtherWrapper<V, E>,
    {
        func(self)
    }
}

#[async_trait]
pub trait AsyncLift<U> {
    type SelfWrapper<T>;
    type OtherWrapper<T, E>;

    async fn async_lift<Func, F, E, V>(self, func: Func) -> Self::OtherWrapper<V, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<V, E>> + Send;
}

#[async_trait]
impl<U, V: Lift<U> + Lift<U, SelfWrapper<U> = V> + Send> AsyncLift<U> for V {
    type SelfWrapper<T> = <V as Lift<U>>::SelfWrapper<T>;
    type OtherWrapper<T, E> = <V as Lift<U>>::OtherWrapper<T, E>;

    async fn async_lift<Func, F, E, W>(self, func: Func) -> Self::OtherWrapper<W, E>
    where
        Func: Fn(Self::SelfWrapper<U>) -> F + Send + Sync,
        F: futures::Future<Output = Self::OtherWrapper<W, E>> + Send,
    {
        func(self).await
    }
}

pub enum CryptoOperation<T: Clone, S: masking::Strategy<T>> {
    Encrypt(Secret<T, S>),
    EncryptOptional(Option<Secret<T, S>>),
    Decrypt(Encryption),
    DecryptOptional(Option<Encryption>),
    BatchEncrypt(FxHashMap<String, Secret<T, S>>),
    BatchDecrypt(FxHashMap<String, Encryption>),
}

use errors::CryptoError;

#[derive(recon_derive::TryGetEnumVariant)]
#[error(CryptoError::EncodingFailed)]
pub enum CryptoOutput<T: Clone, S: masking::Strategy<T>> {
    Operation(crypto::Encryptable<Secret<T, S>>),
    OptionalOperation(Option<crypto::Encryptable<Secret<T, S>>>),
    BatchOperation(FxHashMap<String, crypto::Encryptable<Secret<T, S>>>),
}
