use uuid::Uuid;

pub fn uuid_to_string(uuid: Uuid) -> String {
    let mut ebuf = Uuid::encode_buffer();

    uuid.hyphenated().encode_lower(&mut ebuf).to_string()
}
