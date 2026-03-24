-- GDPR: Remove personally identifiable reviewer data
ALTER TABLE reviews_raw DROP COLUMN IF EXISTS author;
ALTER TABLE reviews_raw DROP COLUMN IF EXISTS author_country;
