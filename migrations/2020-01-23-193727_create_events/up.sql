CREATE TABLE events (
	id VARCHAR PRIMARY KEY NOT NULL,
	"when" VARCHAR NOT NULL,
	what BOOLEAN NOT NULL
);
CREATE INDEX event_timestamp ON events ("when");
