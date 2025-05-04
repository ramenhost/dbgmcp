import string
username = 'R2D2'
is_valid = False
has_digit = False

# Add print statements as directed to help find and fix the logic error.

if len(username) >= 5 and len(username) <= 10:  # Check length.
  is_valid = True

for char in username:   # Loop to check the characters in username.
  if char in string.digits: # Check for a digit (0-9).
    has_digit = True
  elif char not in string.ascii_letters:  # Check for non-letters.
    is_valid = False
  else:
    is_valid = True

if is_valid and has_digit:
  print(f"'{username}' is a valid username.")
else:
  print((f"'{username}' is invalid."))
