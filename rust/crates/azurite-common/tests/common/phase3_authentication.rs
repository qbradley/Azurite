use azurite_common::{
    generateAccountSASSignature, AccountSASPermission, AccountSASPermissions,
    AccountSASPermissionsOrString, AccountSASResourceType, AccountSASResourceTypes,
    AccountSASResourceTypesOrString, AccountSASServices, AccountSASServicesOrString, DateOrString,
    IAccountSASSignatureValues, IIPRange, SASProtocol, SASProtocolOrString, SasIPRange,
    SasIPRangeOrString,
};
use chrono::{Duration, TimeZone, Utc};
use pretty_assertions::assert_eq;
use serde_json::{json, Value};

const CANONICAL_ACCOUNT_SAS_PERMISSIONS: &str = "rwdxlacuptfiy";
const CANONICAL_ACCOUNT_SAS_SERVICES: &str = "btqf";
const CANONICAL_ACCOUNT_SAS_RESOURCE_TYPES: &str = "sco";

#[test]
fn ip_range_to_string_matches_ts_for_single_ranges_and_unvalidated_text() {
    let cases = [
        (
            IIPRange {
                start: "8.8.8.8".to_owned(),
                end: None,
            },
            "8.8.8.8",
        ),
        (
            IIPRange {
                start: "1.1.1.1".to_owned(),
                end: Some("255.255.255.255".to_owned()),
            },
            "1.1.1.1-255.255.255.255",
        ),
        (
            IIPRange {
                start: String::new(),
                end: Some(String::new()),
            },
            "",
        ),
        (
            IIPRange {
                start: "not-an-ip".to_owned(),
                end: Some("still-not-an-ip".to_owned()),
            },
            "not-an-ip-still-not-an-ip",
        ),
    ];

    for (ip_range, expected) in cases {
        assert_eq!(
            azurite_common::authentication::ipRangeToString(&ip_range),
            expected
        );
        assert_eq!(
            azurite_common::authentication::ip_range_to_string(&ip_range),
            expected
        );
    }
}

#[test]
fn account_sas_permissions_parse_sets_each_flag_and_accepts_empty_input() {
    let cases = [
        ("", AccountSASPermissions::default(), ""),
        (
            "r",
            AccountSASPermissions {
                read: true,
                ..Default::default()
            },
            "r",
        ),
        (
            "w",
            AccountSASPermissions {
                write: true,
                ..Default::default()
            },
            "w",
        ),
        (
            "d",
            AccountSASPermissions {
                delete: true,
                ..Default::default()
            },
            "d",
        ),
        (
            "x",
            AccountSASPermissions {
                deleteVersion: true,
                ..Default::default()
            },
            "x",
        ),
        (
            "l",
            AccountSASPermissions {
                list: true,
                ..Default::default()
            },
            "l",
        ),
        (
            "a",
            AccountSASPermissions {
                add: true,
                ..Default::default()
            },
            "a",
        ),
        (
            "c",
            AccountSASPermissions {
                create: true,
                ..Default::default()
            },
            "c",
        ),
        (
            "u",
            AccountSASPermissions {
                update: true,
                ..Default::default()
            },
            "u",
        ),
        (
            "p",
            AccountSASPermissions {
                process: true,
                ..Default::default()
            },
            "p",
        ),
        (
            "t",
            AccountSASPermissions {
                tag: true,
                ..Default::default()
            },
            "t",
        ),
        (
            "f",
            AccountSASPermissions {
                filter: true,
                ..Default::default()
            },
            "f",
        ),
        (
            "i",
            AccountSASPermissions {
                setImmutabilityPolicy: true,
                ..Default::default()
            },
            "i",
        ),
        (
            "y",
            AccountSASPermissions {
                permanentDelete: true,
                ..Default::default()
            },
            "y",
        ),
    ];

    for (input, expected, expected_string) in cases {
        let parsed =
            AccountSASPermissions::parse(input).expect("permission parsing should succeed");
        assert_eq!(
            parsed, expected,
            "unexpected permission flags for {input:?}"
        );
        assert_eq!(parsed.toString(), expected_string);
    }
}

#[test]
fn account_sas_permissions_serialize_in_canonical_ts_order() {
    let parsed =
        AccountSASPermissions::parse("yiftpucalxdwr").expect("reordered permissions should parse");
    assert_eq!(parsed.toString(), CANONICAL_ACCOUNT_SAS_PERMISSIONS);

    let manual = AccountSASPermissions {
        permanentDelete: true,
        setImmutabilityPolicy: true,
        filter: true,
        tag: true,
        process: true,
        update: true,
        create: true,
        add: true,
        list: true,
        deleteVersion: true,
        delete: true,
        write: true,
        read: true,
    };
    assert_eq!(manual.toString(), CANONICAL_ACCOUNT_SAS_PERMISSIONS);
}

#[test]
fn account_sas_permissions_reject_duplicates_unknown_characters_and_any_sentinel() {
    let duplicate = AccountSASPermissions::parse("rr").unwrap_err();
    assert_eq!(duplicate.message, "Duplicated permission character: r");

    let invalid = AccountSASPermissions::parse("rz").unwrap_err();
    assert_eq!(invalid.message, "Invalid permission character: z");

    let sentinel = AccountSASPermissions::parse(AccountSASPermission::Any.as_str()).unwrap_err();
    assert_eq!(sentinel.message, "Invalid permission character: A");
    assert_eq!(AccountSASPermission::Any.to_string(), "AnyPermission");
}

#[test]
fn account_sas_services_parse_sets_each_flag_and_accepts_empty_input() {
    let cases = [
        ("", AccountSASServices::default(), ""),
        (
            "b",
            AccountSASServices {
                blob: true,
                ..Default::default()
            },
            "b",
        ),
        (
            "t",
            AccountSASServices {
                table: true,
                ..Default::default()
            },
            "t",
        ),
        (
            "q",
            AccountSASServices {
                queue: true,
                ..Default::default()
            },
            "q",
        ),
        (
            "f",
            AccountSASServices {
                file: true,
                ..Default::default()
            },
            "f",
        ),
    ];

    for (input, expected, expected_string) in cases {
        let parsed = AccountSASServices::parse(input).expect("service parsing should succeed");
        assert_eq!(parsed, expected, "unexpected service flags for {input:?}");
        assert_eq!(parsed.toString(), expected_string);
    }
}

#[test]
fn account_sas_services_serialize_in_canonical_ts_order() {
    let parsed = AccountSASServices::parse("fqtb").expect("reordered services should parse");
    assert_eq!(parsed.toString(), CANONICAL_ACCOUNT_SAS_SERVICES);

    let manual = AccountSASServices {
        file: true,
        queue: true,
        table: true,
        blob: true,
    };
    assert_eq!(manual.toString(), CANONICAL_ACCOUNT_SAS_SERVICES);
}

#[test]
fn account_sas_services_reject_duplicates_and_unknown_characters() {
    let duplicate = AccountSASServices::parse("bb").unwrap_err();
    assert_eq!(duplicate.message, "Duplicated permission character: b");

    let invalid = AccountSASServices::parse("bz").unwrap_err();
    assert_eq!(invalid.message, "Invalid service character: z");
}

#[test]
fn account_sas_resource_types_parse_sets_each_flag_and_accepts_empty_input() {
    let cases = [
        ("", AccountSASResourceTypes::default(), ""),
        (
            "s",
            AccountSASResourceTypes {
                service: true,
                ..Default::default()
            },
            "s",
        ),
        (
            "c",
            AccountSASResourceTypes {
                container: true,
                ..Default::default()
            },
            "c",
        ),
        (
            "o",
            AccountSASResourceTypes {
                object: true,
                ..Default::default()
            },
            "o",
        ),
    ];

    for (input, expected, expected_string) in cases {
        let parsed =
            AccountSASResourceTypes::parse(input).expect("resource type parsing should succeed");
        assert_eq!(parsed, expected, "unexpected resource flags for {input:?}");
        assert_eq!(parsed.toString(), expected_string);
    }
}

#[test]
fn account_sas_resource_types_serialize_in_canonical_ts_order() {
    let parsed =
        AccountSASResourceTypes::parse("ocs").expect("reordered resource types should parse");
    assert_eq!(parsed.toString(), CANONICAL_ACCOUNT_SAS_RESOURCE_TYPES);

    let manual = AccountSASResourceTypes {
        object: true,
        container: true,
        service: true,
    };
    assert_eq!(manual.toString(), CANONICAL_ACCOUNT_SAS_RESOURCE_TYPES);
}

#[test]
fn account_sas_resource_types_reject_duplicates_unknown_characters_and_any_sentinel() {
    let duplicate = AccountSASResourceTypes::parse("ss").unwrap_err();
    assert_eq!(duplicate.message, "Duplicated permission character: s");

    let invalid = AccountSASResourceTypes::parse("sz").unwrap_err();
    assert_eq!(invalid.message, "Invalid resource type: z");

    let sentinel =
        AccountSASResourceTypes::parse(AccountSASResourceType::Any.as_str()).unwrap_err();
    assert_eq!(sentinel.message, "Invalid resource type: A");
    assert_eq!(AccountSASResourceType::Any.to_string(), "AnyResourceType");
}

#[test]
fn generate_account_sas_signature_20201206_matches_ts_helper_serialization() {
    let values = IAccountSASSignatureValues {
        version: "2020-12-06".to_owned(),
        protocol: Some(SASProtocolOrString::SASProtocol(SASProtocol::HTTPS)),
        startTime: Some(DateOrString::Date(
            Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).unwrap() + Duration::milliseconds(987),
        )),
        expiryTime: DateOrString::Date(
            Utc.with_ymd_and_hms(2024, 1, 2, 4, 5, 6).unwrap() + Duration::milliseconds(123),
        ),
        permissions: AccountSASPermissionsOrString::AccountSASPermissions(AccountSASPermissions {
            permanentDelete: true,
            setImmutabilityPolicy: true,
            filter: true,
            tag: true,
            process: true,
            update: true,
            create: true,
            add: true,
            list: true,
            deleteVersion: true,
            delete: true,
            write: true,
            read: true,
        }),
        ipRange: Some(SasIPRangeOrString::SasIPRange(SasIPRange {
            start: "10.0.0.1".to_owned(),
            end: Some("10.0.0.255".to_owned()),
        })),
        services: AccountSASServicesOrString::AccountSASServices(AccountSASServices {
            file: true,
            queue: true,
            table: true,
            blob: true,
        }),
        resourceTypes: AccountSASResourceTypesOrString::AccountSASResourceTypes(
            AccountSASResourceTypes {
                object: true,
                container: true,
                service: true,
            },
        ),
        encryptionScope: Some("scope-a".to_owned()),
    };

    let (signature, string_to_sign) =
        generateAccountSASSignature(&values, "devstoreaccount1", b"key-value");

    assert_eq!(
        string_to_sign,
        "devstoreaccount1\nrwdxlacuptfiy\nbtqf\nsco\n2024-01-02T03:04:05Z\n2024-01-02T04:05:06Z\n10.0.0.1-10.0.0.255\nhttps\n2020-12-06\nscope-a\n"
    );
    assert_eq!(signature, "plPUZyeQadb4Q21Y5wAktXJty38TqzzraW4qUzfrVrw=");
}

#[test]
fn generate_account_sas_signature_20150405_preserves_raw_strings_and_omits_encryption_scope() {
    let values = IAccountSASSignatureValues {
        version: "2015-04-05".to_owned(),
        protocol: Some(SASProtocolOrString::String("custom,proto".to_owned())),
        startTime: Some(DateOrString::String("start-raw".to_owned())),
        expiryTime: DateOrString::String("expiry-raw".to_owned()),
        permissions: AccountSASPermissionsOrString::String("yr".to_owned()),
        ipRange: Some(SasIPRangeOrString::String("ip-raw".to_owned())),
        services: AccountSASServicesOrString::String("fqb".to_owned()),
        resourceTypes: AccountSASResourceTypesOrString::String("os".to_owned()),
        encryptionScope: Some("ignored-scope".to_owned()),
    };

    let (signature, string_to_sign) =
        generateAccountSASSignature(&values, "account-name", b"second-key");

    assert_eq!(
        string_to_sign,
        "account-name\nyr\nfqb\nos\nstart-raw\nexpiry-raw\nip-raw\ncustom,proto\n2015-04-05\n"
    );
    assert_eq!(signature, "f/pFwWnNvau3pulSfiyODlNTmQr14GluWh3aDIYc9i0=");
}

#[test]
fn generate_account_sas_signature_uses_empty_optional_fields_and_trailing_newline() {
    let values = IAccountSASSignatureValues {
        version: "2020-12-06".to_owned(),
        protocol: None,
        startTime: None,
        expiryTime: DateOrString::String("2024-01-02T04:05:06Z".to_owned()),
        permissions: AccountSASPermissionsOrString::String("rw".to_owned()),
        ipRange: None,
        services: AccountSASServicesOrString::String("bt".to_owned()),
        resourceTypes: AccountSASResourceTypesOrString::String("sc".to_owned()),
        encryptionScope: None,
    };

    let (_signature, string_to_sign) = generateAccountSASSignature(&values, "acct", b"key");

    assert_eq!(
        string_to_sign,
        "acct\nrw\nbt\nsc\n\n2024-01-02T04:05:06Z\n\n\n2020-12-06\n\n"
    );
    assert!(string_to_sign.ends_with('\n'));
}

#[test]
fn signature_value_deserialization_requires_all_non_optional_fields() {
    let required_fields = [
        "version",
        "expiryTime",
        "permissions",
        "services",
        "resourceTypes",
    ];

    for missing_field in required_fields {
        let mut payload = signature_values_json();
        payload
            .as_object_mut()
            .expect("signature payload should be an object")
            .remove(missing_field);

        let error = serde_json::from_value::<IAccountSASSignatureValues>(payload).unwrap_err();
        assert!(
            error.to_string().contains(missing_field),
            "expected missing field error to mention {missing_field}, got {error}"
        );
    }
}

#[test]
fn signature_value_deserialization_preserves_raw_protocol_and_ip_range_strings() {
    let payload = json!({
        "version": "2020-12-06",
        "protocol": "custom-scheme",
        "expiryTime": "2024-01-02T04:05:06Z",
        "permissions": "yr",
        "ipRange": "raw-ip-range",
        "services": "fqb",
        "resourceTypes": "os"
    });

    let parsed: IAccountSASSignatureValues = serde_json::from_value(payload).unwrap();
    assert_eq!(
        parsed.protocol,
        Some(SASProtocolOrString::String("custom-scheme".to_owned()))
    );
    assert_eq!(
        parsed.ipRange,
        Some(SasIPRangeOrString::String("raw-ip-range".to_owned()))
    );
}

fn signature_values_json() -> Value {
    json!({
        "version": "2020-12-06",
        "expiryTime": "2024-01-02T04:05:06Z",
        "permissions": "rw",
        "services": "btqf",
        "resourceTypes": "sco"
    })
}
