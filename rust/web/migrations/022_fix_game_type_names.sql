UPDATE game_versions
SET game_type_id = (SELECT id FROM game_types WHERE name = 'Alhambra')
WHERE game_type_id = (SELECT id FROM game_types WHERE name = 'alhambra-1')
  AND EXISTS (SELECT 1 FROM game_types WHERE name = 'Alhambra');

UPDATE game_type_users
SET game_type_id = (SELECT id FROM game_types WHERE name = 'Alhambra')
WHERE game_type_id = (SELECT id FROM game_types WHERE name = 'alhambra-1')
  AND EXISTS (SELECT 1 FROM game_types WHERE name = 'Alhambra');

DELETE FROM game_types
WHERE name = 'alhambra-1'
  AND EXISTS (SELECT 1 FROM game_types WHERE name = 'Alhambra');

UPDATE game_types SET name = 'Alhambra' WHERE name = 'alhambra-1';

UPDATE game_versions
SET game_type_id = (SELECT id FROM game_types WHERE name = 'Seven Wonders')
WHERE game_type_id = (SELECT id FROM game_types WHERE name = 'seven-wonders-1')
  AND EXISTS (SELECT 1 FROM game_types WHERE name = 'Seven Wonders');

UPDATE game_type_users
SET game_type_id = (SELECT id FROM game_types WHERE name = 'Seven Wonders')
WHERE game_type_id = (SELECT id FROM game_types WHERE name = 'seven-wonders-1')
  AND EXISTS (SELECT 1 FROM game_types WHERE name = 'Seven Wonders');

DELETE FROM game_types
WHERE name = 'seven-wonders-1'
  AND EXISTS (SELECT 1 FROM game_types WHERE name = 'Seven Wonders');

UPDATE game_types SET name = 'Seven Wonders' WHERE name = 'seven-wonders-1';
