try:
    import capstone
    print("capstone", capstone.__version__)
except Exception as e:
    print("no capstone:", e)
