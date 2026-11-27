def can_read(active, admin, member):
    return active or (admin or member)
