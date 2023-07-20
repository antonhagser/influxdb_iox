-- Basic query tests for measurement names that have equals in their names
-- IOX_SETUP: EqualInMeasurements

-- query data
-- IOX_COMPARE: sorted
SELECT * from "measurement=one";



-- projection
-- IOX_COMPARE: sorted
SELECT tag from "measurement=one";
