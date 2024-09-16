#[derive(Debug, Clone, bincode::Encode, bincode::Decode, PartialEq)]
pub enum TestMessage {
    A(u8),
    B(i32),
    C(i64, i64),
    D(u128, u128, u128),
    E(String),
    F(Vec<TestMessage>),
    G(Box<TestMessage>),
    H,
    I(
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
    ),
    Z(
        u8,
        i32,
        i64,
        u128,
        String,
        Vec<TestMessage>,
        Box<TestMessage>,
    ),
}

pub fn z_test_message() -> TestMessage {
    TestMessage::Z(
        1,
        2,
        3,
        4,
        String::from("Hello"),
        std::vec![
            TestMessage::A(100),
            TestMessage::B(100),
            TestMessage::C(100, 100),
            TestMessage::D(100, 100, 100),
        ],
        Box::new(TestMessage::A(100)),
    )
}

pub fn test_messages() -> Vec<TestMessage> {
    std::vec![
        TestMessage::A(100),
        TestMessage::B(100),
        TestMessage::C(100, 100),
        TestMessage::D(100, 100, 100),
        TestMessage::E(String::from("Hello")),
        TestMessage::F(std::vec![
            TestMessage::A(100),
            TestMessage::B(100),
            TestMessage::C(100, 100),
            TestMessage::D(100, 100, 100),
        ]),
        TestMessage::G(Box::new(TestMessage::A(100))),
        TestMessage::H,
        TestMessage::I(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17),
        z_test_message(),
    ]
}
