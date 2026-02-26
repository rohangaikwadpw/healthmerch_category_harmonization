# Troubleshooting Guide

## Database Connection Issues in Installed Application

If the application works fine in development mode but fails to connect to the database when installed, try these solutions:

### Solution 1: Windows Firewall
The most common issue is Windows Firewall blocking the installed application from making network connections.

**Fix:**
1. Open **Windows Defender Firewall with Advanced Security**
2. Click on **Outbound Rules** in the left panel
3. Click **New Rule** in the right panel
4. Select **Program** and click Next
5. Browse to the installed application location (usually `C:\Program Files\Category Harmonization Tool\Category Harmonization Tool.exe`)
6. Select **Allow the connection**
7. Apply to all profiles (Domain, Private, Public)
8. Give it a name like "Category Harmonization Tool - Allow Outbound"
9. Click Finish

### Solution 2: Antivirus Software
Some antivirus programs may block network connections from newly installed applications.

**Fix:**
1. Add the application to your antivirus software's allowlist/whitelist
2. Temporarily disable the antivirus and test if the connection works
3. If it works with antivirus disabled, configure your antivirus to allow the application

### Solution 3: Run as Administrator
Sometimes elevated permissions are needed for network operations.

**Fix:**
1. Right-click on the application shortcut or executable
2. Select **Run as administrator**
3. Test the database connection

### Solution 4: Check Network Connectivity
Ensure your computer can reach the database server.

**Test:**
1. Open PowerShell
2. Run: `Test-NetConnection -ComputerName promohub.cebrdrk3gama.ap-south-1.rds.amazonaws.com -Port 5432`
3. Check if `TcpTestSucceeded` is `True`

### Solution 5: Reinstall Application
If the above solutions don't work, try uninstalling and reinstalling the application.

**Steps:**
1. Uninstall from Windows Settings > Apps
2. Delete any remaining files in `C:\Program Files\Category Harmonization Tool`
3. Reinstall using the latest installer

### Getting Help
If none of these solutions work, please check the application logs:
- The application prints detailed error messages to the console
- Look for messages starting with "=== DATABASE CONNECTION FAILED ==="
- The error message will indicate the specific cause

### Database Connection Error: "role is not permitted to log in"
If you see this error, the database user needs LOGIN permission.

**Fix (requires database admin access):**
```sql
ALTER ROLE categoryharmonization WITH LOGIN;
```

Or contact your database administrator to grant this permission.
