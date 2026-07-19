use chrono::{DateTime, Utc};
use prost_types::Timestamp;

/// Convert a `prost_types::Timestamp` to `chrono::DateTime<Utc>`.
pub fn proto_to_datetime(ts: &Option<Timestamp>) -> Option<DateTime<Utc>> {
    ts.as_ref().map(|ts| {
        let secs = ts.seconds;
        let nsecs = ts.nanos as u32;
        DateTime::from_timestamp(secs, nsecs).unwrap()
    })
}

/// Convert a `chrono::DateTime<Utc>` to a `prost_types::Timestamp`.
pub fn datetime_to_proto(dt: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

/// Convert an `Option<chrono::DateTime<Utc>>` to `prost_types::Timestamp`.
pub fn option_datetime_to_proto(dt: Option<DateTime<Utc>>) -> Option<Timestamp> {
    dt.map(datetime_to_proto)
}

/// Get the current time as a `prost_types::Timestamp`.
pub fn utc_now_proto() -> Timestamp {
    datetime_to_proto(Utc::now())
}

/// Convert `std::time::Duration` to `prost_types::Duration`.
pub fn duration_to_proto(d: &std::time::Duration) -> prost_types::Duration {
    prost_types::Duration {
        seconds: d.as_secs() as i64,
        nanos: d.subsec_nanos() as i32,
    }
}

/// Convert `prost_types::Duration` to `std::time::Duration`.
pub fn proto_to_duration(d: &prost_types::Duration) -> std::time::Duration {
    std::time::Duration::new(d.seconds as u64, d.nanos as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_proto_roundtrip() {
        let dt = Utc::now();
        let proto = datetime_to_proto(dt);
        let dt2 = proto_to_datetime(&Some(proto)).unwrap();
        assert_eq!(dt.timestamp(), dt2.timestamp());
        assert_eq!(dt.timestamp_subsec_nanos(), dt2.timestamp_subsec_nanos());
    }

    #[test]
    fn test_proto_to_datetime_none() {
        assert!(proto_to_datetime(&None).is_none());
    }

    #[test]
    fn test_duration_proto_roundtrip() {
        let d = std::time::Duration::from_secs(3600);
        let proto = duration_to_proto(&d);
        let d2 = proto_to_duration(&proto);
        assert_eq!(d, d2);
    }

    #[test]
    fn test_utc_now_proto() {
        let proto = utc_now_proto();
        assert!(proto.seconds > 0);
    }

    #[test]
    fn test_option_datetime_to_proto_some() {
        let dt = Utc::now();
        let proto = option_datetime_to_proto(Some(dt)).unwrap();
        assert_eq!(proto.seconds, dt.timestamp());
    }

    #[test]
    fn test_option_datetime_to_proto_none() {
        assert!(option_datetime_to_proto(None).is_none());
    }
}
