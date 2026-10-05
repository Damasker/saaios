try:
    import capstone
    print("capstone", capstone.__version__)
except Exception as e:
    print("no-capstone", type(e).__name__, e)
