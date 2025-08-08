use validator::Validate;

// Silly intermediate type because validator crate has changed.
// I get why this is nice. But I have to combine all user data to single thing to fully extract the
// potential instead of the raw Rust approach. Idk, the example is weird too
#[derive(Debug, Validate)]
struct UnverifiedSubscriberEmail {
    #[validate(email)]
    email: String,
}

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<SubscriberEmail, String> {
        let input = UnverifiedSubscriberEmail { email: s.clone() };

        if input.validate().is_err() {
            Err(format!("{s} is not a valid email."))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use claims::assert_err;
    use fake::{
        Fake,
        faker::internet::en::SafeEmail,
        rand::{SeedableRng, rngs::StdRng},
    };
    use quickcheck::Arbitrary;

    use crate::domain::SubscriberEmail;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl Arbitrary for ValidEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ardnysdomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[quickcheck_macros::quickcheck]
    fn valid_emails_are_parsed_successfully(valid_email: ValidEmailFixture) -> bool {
        // dbg!(&valid_email.0);  // run with cargo test valis_emails -- --nocapture to see them
        // emails
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
