CREATE TABLE example_products (
    product_id SERIAL PRIMARY KEY,
    product_name TEXT NOT NULL,
    description TEXT,
    last_updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO example_products(product_name, description, last_updated_at) VALUES
('Pencil', 'Utensil used for writing and often works best on paper', NOW()),
('Laptop Stand', 'Elevated platform for laptops, enhancing ergonomics', NOW()),
('Desk Lamp', 'Illumination device for workspaces, often adjustable', NOW()),
('Bluetooth Speaker', 'Portable audio device with wireless connectivity', NOW()),
('Water Bottle', 'Reusable container for liquids, often insulated', NOW()),
('Backpack', 'Storage solution for carrying personal items on one’s back', NOW()),
('Wireless Mouse', 'Pointing device without the need for a physical connection', NOW()),
('Plant Pot', 'Container for holding plants, often with drainage', NOW()),
('Sunglasses', 'Protective eyewear to shield eyes from UV rays', NOW()),
('Notebook', 'Bound sheets of paper for note-taking or sketching', NOW()),
('Stylus Pen', 'Tool for touchscreen devices, mimics finger touch', NOW()),
('Travel Mug', 'Insulated container for beverages on-the-go', NOW()),
('Phone Charger', 'Device to replenish the battery of mobile phones', NOW()),
('Yoga Mat', 'Cushioned surface for practicing yoga or exercise', NOW()),
('Wall Clock', 'Time-telling device meant to hang on walls', NOW()),
('Keychain', 'Small device for holding keys together', NOW()),
('Desk Organizer', 'Tool for sorting and storing desk items', NOW()),
('Earbuds', 'Small headphones that fit directly inside the ear', NOW()),
('Calendar', 'Physical representation of days and months, often used for scheduling', NOW()),
('Umbrella', 'Protective gear against rain or intense sun', NOW()),
('Hand Sanitizer', 'Liquid or gel used to decrease infectious agents on hands', NOW()),
('Sketchbook', 'Paper-filled book used for drawing or painting', NOW()),
('Flash Drive', 'Portable storage device for digital files', NOW()),
('Tablet Holder', 'Stand or grip for holding tablets or e-readers', NOW()),
('Shampoo', 'Hair care product designed to cleanse the scalp and hair', NOW()),
('Wristwatch', 'Time-telling device worn around the wrist', NOW()),
('Basketball', 'Spherical sporting equipment used in basketball games', NOW()),
('Guitar Picks', 'Small flat tool used to strum or pick a guitar', NOW()),
('Thermal Flask', 'Insulated bottle for keeping beverages hot or cold', NOW()),
('Slippers', 'Soft and light footwear intended for indoor use', NOW()),
('Easel', 'Upright support for artists to display or work on canvases', NOW()),
('Bicycle Helmet', 'Protective headgear for cyclists', NOW()),
('Candle Holder', 'Accessory to safely hold candles when they burn', NOW()),
('Cutting Board', 'Durable board on which to place materials for cutting', NOW()),
('Gardening Gloves', 'Handwear for protection during gardening tasks', NOW()),
('Alarm Clock', 'Time-telling device with a feature to sound at a specified time', NOW()),
('Spatula', 'Flat tool used in cooking for flipping or spreading', NOW()),
('Jigsaw Puzzle', 'Picture printed on cardboard or wood and cut into pieces to be reassembled', NOW()),
('Hammock', 'Sling made of fabric or netting, suspended between two points for relaxation', NOW()),
('Luggage Tag', 'Accessory attached to luggage for identification purposes', NOW())
;
