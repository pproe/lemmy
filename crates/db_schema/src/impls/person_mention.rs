use crate::{
  newtypes::{CommentId, PersonId, PersonMentionId},
  source::person_mention::*,
  traits::Crud,
};
use diesel::{dsl::*, result::Error, *};

impl Crud for PersonMention {
  type InsertForm = PersonMentionInsertForm;
  type UpdateForm = PersonMentionUpdateForm;
  type IdType = PersonMentionId;
  fn read(conn: &mut PgConnection, person_mention_id: PersonMentionId) -> Result<Self, Error> {
    use crate::schema::person_mention::dsl::*;
    person_mention.find(person_mention_id).first::<Self>(conn)
  }

  fn create(
    conn: &mut PgConnection,
    person_mention_form: &Self::InsertForm,
  ) -> Result<Self, Error> {
    use crate::schema::person_mention::dsl::*;
    // since the return here isnt utilized, we dont need to do an update
    // but get_result doesnt return the existing row here
    insert_into(person_mention)
      .values(person_mention_form)
      .on_conflict((recipient_id, comment_id))
      .do_update()
      .set(person_mention_form)
      .get_result::<Self>(conn)
  }

  fn update(
    conn: &mut PgConnection,
    person_mention_id: PersonMentionId,
    person_mention_form: &Self::UpdateForm,
  ) -> Result<Self, Error> {
    use crate::schema::person_mention::dsl::*;
    diesel::update(person_mention.find(person_mention_id))
      .set(person_mention_form)
      .get_result::<Self>(conn)
  }
}

impl PersonMention {
  pub fn mark_all_as_read(
    conn: &mut PgConnection,
    for_recipient_id: PersonId,
  ) -> Result<Vec<PersonMention>, Error> {
    use crate::schema::person_mention::dsl::*;
    diesel::update(
      person_mention
        .filter(recipient_id.eq(for_recipient_id))
        .filter(read.eq(false)),
    )
    .set(read.eq(true))
    .get_results::<Self>(conn)
  }

  pub fn read_by_comment_and_person(
    conn: &mut PgConnection,
    for_comment_id: CommentId,
    for_recipient_id: PersonId,
  ) -> Result<Self, Error> {
    use crate::schema::person_mention::dsl::*;
    person_mention
      .filter(comment_id.eq(for_comment_id))
      .filter(recipient_id.eq(for_recipient_id))
      .first::<Self>(conn)
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    source::{
      comment::*,
      community::{Community, CommunityInsertForm},
      instance::Instance,
      person::*,
      person_mention::*,
      post::*,
    },
    traits::Crud,
    utils::establish_unpooled_connection,
  };
  use serial_test::serial;

  #[test]
  #[serial]
  fn test_crud() {
    let conn = &mut establish_unpooled_connection();

    let inserted_instance = Instance::create(conn, "my_domain.tld").unwrap();

    let new_person = PersonInsertForm::builder()
      .name("terrylake".into())
      .public_key("pubkey".to_string())
      .instance_id(inserted_instance.id)
      .build();

    let inserted_person = Person::create(conn, &new_person).unwrap();

    let recipient_form = PersonInsertForm::builder()
      .name("terrylakes recipient".into())
      .public_key("pubkey".to_string())
      .instance_id(inserted_instance.id)
      .build();

    let inserted_recipient = Person::create(conn, &recipient_form).unwrap();

    let new_community = CommunityInsertForm::builder()
      .name("test community lake".to_string())
      .title("nada".to_owned())
      .public_key("pubkey".to_string())
      .instance_id(inserted_instance.id)
      .build();

    let inserted_community = Community::create(conn, &new_community).unwrap();

    let new_post = PostInsertForm::builder()
      .name("A test post".into())
      .creator_id(inserted_person.id)
      .community_id(inserted_community.id)
      .build();

    let inserted_post = Post::create(conn, &new_post).unwrap();

    let comment_form = CommentInsertForm::builder()
      .content("A test comment".into())
      .creator_id(inserted_person.id)
      .post_id(inserted_post.id)
      .build();

    let inserted_comment = Comment::create(conn, &comment_form, None).unwrap();

    let person_mention_form = PersonMentionInsertForm {
      recipient_id: inserted_recipient.id,
      comment_id: inserted_comment.id,
      read: None,
    };

    let inserted_mention = PersonMention::create(conn, &person_mention_form).unwrap();

    let expected_mention = PersonMention {
      id: inserted_mention.id,
      recipient_id: inserted_mention.recipient_id,
      comment_id: inserted_mention.comment_id,
      read: false,
      published: inserted_mention.published,
    };

    let read_mention = PersonMention::read(conn, inserted_mention.id).unwrap();

    let person_mention_update_form = PersonMentionUpdateForm { read: Some(false) };
    let updated_mention =
      PersonMention::update(conn, inserted_mention.id, &person_mention_update_form).unwrap();
    Comment::delete(conn, inserted_comment.id).unwrap();
    Post::delete(conn, inserted_post.id).unwrap();
    Community::delete(conn, inserted_community.id).unwrap();
    Person::delete(conn, inserted_person.id).unwrap();
    Person::delete(conn, inserted_recipient.id).unwrap();
    Instance::delete(conn, inserted_instance.id).unwrap();

    assert_eq!(expected_mention, read_mention);
    assert_eq!(expected_mention, inserted_mention);
    assert_eq!(expected_mention, updated_mention);
  }
}
