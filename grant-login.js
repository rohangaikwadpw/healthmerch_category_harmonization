const { Client } = require('pg');

// Get credentials from command line arguments or use defaults
const adminUser = process.argv[2] || 'postgres';
const adminPassword = process.argv[3];

if (!adminPassword) {
  console.error('Usage: node grant-login.js <admin_username> <admin_password>');
  console.error('Example: node grant-login.js postgres mypassword');
  process.exit(1);
}

console.log('=== Grant LOGIN Permission to categoryharmonization ===\n');
console.log(`Connecting as: ${adminUser}`);

const connectionString = `postgresql://${adminUser}:${adminPassword}@promohub.cebrdrk3gama.ap-south-1.rds.amazonaws.com:5432/postgres`;

const client = new Client({
  connectionString,
  ssl: {
    rejectUnauthorized: false
  }
});

async function grantLogin() {
  try {
    console.log('\nConnecting to database...');
    await client.connect();
    console.log('✓ Connected successfully!');
    
    console.log('\nGranting LOGIN permission to categoryharmonization role...');
    await client.query('ALTER ROLE categoryharmonization WITH LOGIN;');
    console.log('✓ LOGIN permission granted successfully!');
    
    console.log('\nVerifying the change...');
    const result = await client.query(`
      SELECT rolname, rolcanlogin 
      FROM pg_roles 
      WHERE rolname = 'categoryharmonization';
    `);
    
    if (result.rows.length > 0) {
      console.log('Role details:', result.rows[0]);
      if (result.rows[0].rolcanlogin) {
        console.log('\n✓ SUCCESS: categoryharmonization can now log in!');
      } else {
        console.log('\n✗ WARNING: Login permission was not set correctly.');
      }
    }
    
  } catch (error) {
    console.error('\n✗ Error:', error.message);
    if (error.code) {
      console.error('Error code:', error.code);
    }
    process.exit(1);
  } finally {
    await client.end();
  }
}

grantLogin();
